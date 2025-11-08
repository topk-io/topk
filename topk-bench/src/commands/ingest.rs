use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
use tokio::sync::Notify;
use tokio::task::{JoinHandle, JoinSet};
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
        ProviderArg::TopkRs => TopkRsProvider::new().await?,
        ProviderArg::TopkPy => TopkPyProvider::new().await?,
        ProviderArg::TpufPy => TpufPyProvider::new().await?,
    };

    // Setup provider
    provider.setup(args.collection.clone()).await?;

    // Ping provider
    // First ping to ensure the provider is ready
    info!("Pinging provider");
    provider.ping(args.collection.clone()).await?;
    // Then measure
    for _ in 0..3 {
        let latency = provider.ping(args.collection.clone()).await?;
        info!("Ping latency: {:?}", latency);
    }

    let ready = Arc::new(Notify::new());

    // Spawn batch producer
    let rx = spawn_batch_producer(
        dataset_path,
        args.batch_size,
        args.concurrency,
        ready.clone(),
    )?;

    // Spawn writers
    let writers = spawn_writers(
        provider.clone(),
        args.collection.clone(),
        rx,
        args.concurrency,
        parse_bench_01,
        ready.clone(),
    );

    // Spawn metrics reporter
    let stats = spawn_metrics_reporter(ready.clone());

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
            // Close provider
            if let Err(error) = provider.close().await {
                error!(?error, "Failed to close provider");
            }

            // Export metrics
            if let Err(error) = export_metrics(BUCKET_NAME, metadata, &ingest_id).await {
                error!(?error, "Failed to export metrics");
            }

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
    concurrency: usize,
    ready: Arc<Notify>,
) -> anyhow::Result<Receiver<RecordBatch>> {
    let (tx, rx) = async_channel::unbounded();
    let file = File::open(input)?;
    let mut batch_reader = ParquetRecordBatchReader::try_new(file, batch_size)?;

    std::thread::spawn(move || {
        let mut produced = 0;
        while let Some(batch) = batch_reader.next() {
            let batch = batch.expect("Failed to read batch");

            produced += 1;
            if produced == concurrency {
                ready.notify_waiters();
            }

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
    collection: String,
    rx: Receiver<RecordBatch>,
    concurrency: usize,
    parser: fn(RecordBatch) -> Vec<Document>,
    ready: Arc<Notify>,
) -> tokio::task::JoinHandle<()> {
    // Spawn collection writer tasks
    let mut writers = JoinSet::new();

    for _ in 0..concurrency {
        let rx = rx.clone();
        let provider = provider.clone();
        let collection = collection.clone();
        let ready = ready.clone();

        writers.spawn(async move {
            // Wait for batches to be produced
            ready.notified().await;

            // Spawn freshness tasks
            let mut freshness_tasks = JoinSet::new();

            // Writer task
            loop {
                let recv_start = Instant::now();
                let batch = match rx.recv().await {
                    Ok(batch) => batch,
                    Err(_) => break, // Channel closed
                };
                let recv_latency = recv_start.elapsed();
                histogram!("bench.ingest.recv_latency_ms").record(recv_latency.as_millis() as f64);

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
                        .upsert(
                            collection.clone(),
                            documents.into_iter().map(|doc| doc.into()).collect(),
                        )
                        .await;

                    counter!("bench.ingest.requests").increment(1);
                    match result {
                        Ok(res) => {
                            counter!("bench.ingest.oks").increment(1);
                            counter!("bench.ingest.upserted_docs").increment(doc_count as u64);
                            counter!("bench.ingest.upserted_bytes").increment(byte_size as u64);
                            let latency = s.elapsed();
                            histogram!("bench.ingest.latency_ms")
                                .record(latency.as_millis() as f64);
                            debug!(?doc_count, ?res, ?latency, "Upserted documents");

                            // After a successful upsert, spawn a freshness task
                            let collection = collection.clone();
                            freshness_tasks.spawn(async move {
                                if let Err(error) =
                                    freshness_task(provider, collection, max_id).await
                                {
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
fn spawn_metrics_reporter(ready: Arc<Notify>) -> JoinHandle<()> {
    tokio::task::spawn(async move {
        // Wait for writers to be ready
        info!("Waiting for writers to be ready...");
        ready.notified().await;

        let mut ticker = tokio::time::interval(Duration::from_secs(1));
        loop {
            ticker.tick().await;
            let metrics = read_snapshot().await;
            let get_count = |name: &str| metrics.get(name).map(|m| m.count()).unwrap_or_default();
            let get_sum = |name: &str| metrics.get(name).map(|m| m.sum()).unwrap_or_default();
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
            let total_upserted_bytes = get_sum("bench.ingest.upserted_bytes");
            let avg_latency = metrics
                .get("bench.ingest.latency_ms")
                .map(|m| m.mean())
                .unwrap_or_default();

            let max_freshness_latency = metrics
                .get("bench.ingest.freshness_latency_ms")
                .map(|m| m.max())
                .unwrap_or_default();

            let avg_recv_latency = metrics
                .get("bench.ingest.recv_latency_ms")
                .map(|m| m.mean())
                .unwrap_or_default();

            let max_recv_latency = metrics
                .get("bench.ingest.recv_latency_ms")
                .map(|m| m.max())
                .unwrap_or_default();

            if availability.is_nan() {
                println!("stats] Waiting for metrics...");
                continue;
            }

            println!(
                "stats] {} {} Throughput: {}, {}, {}, Latency: {}, {}, Freshness: {}, Recv: {}, {}",
                // Availability
                match availability {
                    _ if availability == 100.0 => format!("100%").green().bold(),
                    _ if availability > 99.0 => format!("{:.2}%", availability).yellow().bold(),
                    _ if availability.is_nan() => format!("...").bold(),
                    _ => format!("{:.2}%", availability).red().bold(),
                },
                // Total
                format!("{}MB", (total_upserted_bytes / 1024.0 / 1024.0).floor()).bold(),
                // Throughput
                format!("{:.2} reqs/s", get_rate("bench.ingest.requests"))
                    .yellow()
                    .bold(),
                format!(
                    "{:.2}k docs/s",
                    get_rate("bench.ingest.upserted_docs") / 1000.0
                )
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
                format!("p99={:.2}ms", get_quantile("bench.ingest.latency_ms", 0.99))
                    .magenta()
                    .bold(),
                // Freshness
                format!("max={:.2}ms", max_freshness_latency).bold(),
                // Recv
                format!("avg={:.2}ms", avg_recv_latency).bold(),
                format!("max={:.2}ms", max_recv_latency).bold(),
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
    // Ensure the directory exists
    std::fs::create_dir_all(Path::new(&out).parent().unwrap())?;
    let mut file = tokio::fs::File::create(&out).await?;
    tokio::io::copy(&mut data, &mut file).await?;
    let duration = start.elapsed();

    info!(?out, ?duration, "Dataset downloaded");

    Ok(PathBuf::from(out))
}

async fn freshness_task(
    provider: impl ProviderLike + Send + Sync + Clone + 'static,
    collection: String,
    id: String,
) -> anyhow::Result<()> {
    let first = provider.query_by_id(collection.clone(), id.clone()).await?;

    // If found immediately, record 0 latency since it's the benchmarking
    // tool that took some time to send the first request
    if first.is_some() {
        histogram!("bench.ingest.freshness_latency_ms").record(0.);
        return Ok(());
    }

    let start = Instant::now();
    loop {
        let res = provider.query_by_id(collection.clone(), id.clone()).await?;
        if res.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let latency = start.elapsed();

    histogram!("bench.ingest.freshness_latency_ms").record(latency.as_millis() as f64);
    Ok(())
}
