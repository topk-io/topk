use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use arrow_array::RecordBatch;
use async_channel::Receiver;
use clap::Parser;
use colored::Colorize;
use metrics::{counter, histogram};
use parquet::arrow::arrow_reader::ParquetRecordBatchReader;
use parquet::file::metadata::KeyValue;
use rand::{thread_rng, Rng};
use tokio::signal::ctrl_c;
use tokio::task::JoinSet;
use tracing::{debug, error, info};

use crate::commands::{ProviderArg, BUCKET_NAME};
use crate::data::{parse_bench_01, Document};
use crate::providers::topk_py::TopkPyProvider;
use crate::providers::topk_rs::TopkRsProvider;
use crate::providers::tpuf_py::TpufPyProvider;
use crate::providers::ProviderLike;
use crate::s3::new_client;
use crate::telemetry::metrics::{export_metrics, read_snapshot};

#[derive(Parser, Debug, Clone)]
pub struct IngestArgs {
    #[arg(long, help = "Target collection")]
    pub(crate) collection: String,

    #[arg(short, long, help = "Target collection")]
    pub(crate) provider: ProviderArg,

    #[arg(short, long, help = "Number of documents per upsert request")]
    pub(crate) batch_size: usize,

    #[arg(short, long, help = "Number of concurrent writers")]
    pub(crate) concurrency: usize,

    #[arg(short, long, help = "Name of the dataset to ingest")]
    pub(crate) dataset: Option<String>,

    #[arg(short, long, help = "Input file to ingest")]
    pub(crate) input: Option<String>,
}

pub async fn run(args: IngestArgs) -> anyhow::Result<()> {
    // Generate ingest ID
    let ingest_id = uuid::Uuid::new_v4()
        .to_string()
        .chars()
        .take(8)
        .collect::<String>();

    info!("Starting ingest: {:?} with ID: {}", args, ingest_id);

    // Determine dataset path
    let dataset_path = if let Some(ref dataset) = args.dataset {
        pull_dataset(BUCKET_NAME, &dataset).await?
    } else if let Some(ref input) = args.input {
        PathBuf::from(input)
    } else {
        anyhow::bail!("Either dataset or input file must be provided");
    };

    // Create provider
    let provider = match args.provider {
        ProviderArg::TopkRs => TopkRsProvider::new(args.collection).await?,
        ProviderArg::TopkPy => TopkPyProvider::new(args.collection).await?,
        ProviderArg::TpufPy => TpufPyProvider::new(args.collection).await?,
    };

    // Setup provider
    provider.setup().await?;

    // Ping provider
    // First ping to ensure the provider is ready
    provider.ping().await?;
    // Then measure
    for _ in 0..3 {
        let latency = provider.ping().await?;
        info!("Ping latency: {:?}", latency);
    }

    // Spawn batch producer
    let rx = spawn_batch_producer(dataset_path, args.batch_size)?;

    // Spawn writers
    let writers = spawn_writers(provider.clone(), rx, args.concurrency, parse_bench_01);

    // Spawn metrics reporter
    let stats = spawn_metrics_reporter();

    // Build metrics metadata
    let metadata = vec![
        KeyValue::new("provider".into(), format!("{:?}", args.provider)),
        KeyValue::new("batch_size".into(), format!("{:?}", args.batch_size)),
        KeyValue::new("concurrency".into(), format!("{:?}", args.concurrency)),
        KeyValue::new("dataset".into(), format!("{:?}", args.dataset)),
    ];

    let start = Instant::now();
    tokio::select! {
        res = writers => {
            if let Err(error) = res {
                error!(?error, "Writer tasks exited");
            }
        }
        res = stats => {
            error!(?res, "Metrics reporter exited");
        }
        _ = ctrl_c() => {
            provider.close().await?;

            // export (partial) metrics
            export_metrics(BUCKET_NAME, metadata, &ingest_id).await?;

            std::process::exit(128 + 2);
        }
    }

    let duration = start.elapsed();
    info!("Ingest completed in {:.2}s", duration.as_secs_f64());

    // Export final metrics
    export_metrics(BUCKET_NAME, metadata, &ingest_id).await?;

    Ok(())
}

// Spawn batch producer task
fn spawn_batch_producer(
    input: PathBuf,
    batch_size: usize,
) -> anyhow::Result<Receiver<RecordBatch>> {
    let (tx, rx) = async_channel::unbounded();
    let file = File::open(input)?;
    let mut batch_reader = ParquetRecordBatchReader::try_new(file, batch_size)?;

    std::thread::spawn(move || {
        while let Some(batch) = batch_reader.next() {
            let batch = batch.expect("Failed to read batch");

            futures::executor::block_on(async {
                if tx.send(batch).await.is_err() {
                    // Receiver dropped, stop streaming
                    return;
                }
            });
        }
    });

    Ok(rx)
}

// Spawn writer tasks
fn spawn_writers(
    provider: impl ProviderLike + Send + Sync + Clone + 'static,
    rx: Receiver<RecordBatch>,
    concurrency: usize,
    parser: fn(RecordBatch) -> Vec<Document>,
) -> tokio::task::JoinHandle<()> {
    // Spawn collection writer tasks
    let mut writers = JoinSet::new();

    for _ in 0..concurrency {
        let rx = rx.clone();
        let provider = provider.clone();

        writers.spawn(async move {
            // Spawn freshness tasks
            let mut freshness_tasks = JoinSet::new();

            // Writer task
            while let Ok(batch) = rx.recv().await {
                let doc_count = batch.num_rows();
                let provider = provider.clone();

                loop {
                    // Parse batch
                    let documents = parser(batch.clone());

                    // Calculate encoded size from parsed documents
                    let byte_size: usize = documents.iter().map(|doc| doc.encoded_len()).sum();

                    // Calculate max ID from batch
                    let max_id = documents
                        .iter()
                        .map(|doc| {
                            doc.get("id")
                                .unwrap()
                                .as_string()
                                .unwrap()
                                .parse::<u64>()
                                .expect("Failed to parse ID as u64")
                        })
                        .max()
                        .expect("Failed to find max ID")
                        .to_string();

                    let s = Instant::now();
                    let result = provider
                        .upsert(documents.into_iter().map(|doc| doc.into()).collect())
                        .await;

                    counter!("bench.ingest.requests").increment(1);
                    match result {
                        Ok(res) => {
                            counter!("bench.ingest.oks").increment(1);
                            counter!("bench.ingest.upserted_items").increment(doc_count as u64);
                            counter!("bench.ingest.upserted_bytes").increment(byte_size as u64);
                            let latency = s.elapsed();
                            histogram!("bench.ingest.latency_ms")
                                .record(latency.as_millis() as f64);
                            debug!(?doc_count, ?res, ?latency, "Upserted documents");

                            // After a successful upsert, spawn a freshness task
                            freshness_tasks.spawn(async move {
                                if let Err(error) = freshness_task(provider.clone(), max_id).await {
                                    error!(?error, "Failed to spawn freshness task");
                                }
                            });

                            break;
                        }
                        Err(error) => {
                            let latency = s.elapsed();
                            counter!("bench.ingest.errors").increment(1);
                            histogram!("bench.ingest.latency_ms")
                                .record(latency.as_millis() as f64);
                            error!(?latency, "Failed to upsert documents: {:#?}", error);
                            // Sleep
                            let jitter = thread_rng().gen_range(10..100);
                            tokio::time::sleep(Duration::from_millis(jitter)).await;
                        }
                    }
                }
            }

            // Wait for freshness tasks
            while let Some(res) = freshness_tasks.join_next().await {
                match res {
                    Ok(_) => continue,
                    Err(error) => {
                        error!(?error, "Freshness task panicked");
                    }
                }
            }
        });
    }

    // Spawn writer clients
    let writers = tokio::task::spawn(async move {
        while let Some(res) = writers.join_next().await {
            match res {
                Ok(_) => continue,
                Err(error) => {
                    error!(?error, "Writer task panicked");
                    break;
                }
            }
        }

        writers.shutdown().await;
    });

    writers
}

// metrics reporter task
fn spawn_metrics_reporter() -> tokio::task::JoinHandle<()> {
    tokio::task::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(1));
        loop {
            ticker.tick().await;
            let metrics = read_snapshot().await;
            let get_count = |name: &str| metrics.get(name).map(|m| m.count()).unwrap_or_default();
            let get_rate = |name: &str| {
                metrics
                    .get(name)
                    .map(|m| m.rate_per_sec())
                    .unwrap_or_default()
            };
            let get_quantile = |name: &str, quantile: f64| {
                metrics
                    .get(name)
                    .map(|m| m.quantile(quantile))
                    .unwrap_or_default()
            };

            let requests = get_count("bench.ingest.requests");
            let errors = get_count("bench.ingest.errors");

            let availability = (1.0 - errors as f64 / requests as f64) * 100.0;
            let bytes_per_second = get_rate("bench.ingest.upserted_bytes");
            let avg_latency = metrics
                .get("bench.ingest.latency_ms")
                .map(|m| m.mean())
                .unwrap_or_default();

            let avg_freshness_latency = metrics
                .get("bench.ingest.freshness_latency_ms")
                .map(|m| m.mean())
                .unwrap_or_default();

            if availability.is_nan() {
                println!("stats] Waiting for metrics...");
                continue;
            }

            println!(
                "stats] Availability: {}, Throughput: {}, {}, {}, Latency: {}, {}, {}, {}, Freshness: {}",
                // Availability
                match availability {
                    _ if availability == 100.0 => format!("100%").green().bold(),
                    _ if availability > 99.0 => format!("{:.2}%", availability).yellow().bold(),
                    _ if availability.is_nan() => format!("...").bold(),
                    _ => format!("{:.2}%", availability).red().bold(),
                },
                // Throughput
                format!("{:.2} reqs/s", get_rate("bench.ingest.requests"))
                    .yellow()
                    .bold(),
                format!("{:.2} items/s", get_rate("bench.ingest.upserted_items"))
                    .blue()
                    .bold(),
                match bytes_per_second {
                    _ if bytes_per_second < 1024.0 => format!("{:.2} B/s", bytes_per_second),
                    _ if bytes_per_second < (1024.0 * 1024.0) =>
                        format!("{:.2} KB/s", bytes_per_second / 1024.0),
                    _ => format!("{:.2} MB/s", bytes_per_second / (1024.0 * 1024.0)),
                }
                .magenta()
                .bold(),
                // Latency
                format!("avg={:.2}ms", avg_latency).yellow().bold(),
                format!("p50={:.2}", get_quantile("bench.ingest.latency_ms", 0.5))
                    .blue()
                    .bold(),
                format!("p90={:.2}ms", get_quantile("bench.ingest.latency_ms", 0.9))
                    .magenta()
                    .bold(),
                format!("p99={:.2}ms", get_quantile("bench.ingest.latency_ms", 0.99))
                    .cyan()
                    .bold(),
                format!("avg={:.2}ms", avg_freshness_latency).yellow().bold(),
            );
        }
    })
}

async fn pull_dataset(bucket: &str, key: &str) -> anyhow::Result<PathBuf> {
    info!(?bucket, ?key, "Pulling dataset");

    // Ensure the /tmp/topk-bench directory exists first
    let dir = Path::new("/tmp/topk-bench");
    if !dir.exists() {
        std::fs::create_dir_all(dir)?;
    }

    let out = format!("/tmp/topk-bench/{key}");
    if Path::new(&out).exists() {
        info!(?out, "Dataset already downloaded");
        return Ok(PathBuf::from(out));
    }

    // Download dataset
    let s3 = new_client()?;

    let start = Instant::now();
    let resp = s3.get_object().bucket(bucket).key(key).send().await?;
    let mut data = resp.body.into_async_read();
    let mut file = tokio::fs::File::create(&out).await?;
    tokio::io::copy(&mut data, &mut file).await?;
    let duration = start.elapsed();

    info!(?out, ?duration, "Dataset downloaded");

    Ok(PathBuf::from(out))
}

async fn freshness_task(
    provider: impl ProviderLike + Send + Sync + Clone + 'static,
    id: String,
) -> anyhow::Result<()> {
    let first = provider.query_by_id(id.clone()).await?;

    // If found immediately, record 0 latency since it's the benchmarking
    // tool that took some time to send the first request
    if first.is_some() {
        histogram!("bench.ingest.freshness_latency_ms").record(0.);
        return Ok(());
    }

    let start = Instant::now();
    loop {
        let res = provider.query_by_id(id.clone()).await?;
        if res.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let latency = start.elapsed();

    histogram!("bench.ingest.freshness_latency_ms").record(latency.as_millis() as f64);
    Ok(())
}
