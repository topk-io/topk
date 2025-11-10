use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Context;
use arrow_array::RecordBatch;
use async_channel::Receiver;
use clap::Parser;
use colored::Colorize;
use metrics::{counter, histogram};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::file::metadata::KeyValue;
use rand::prelude::*;
use rand::{thread_rng, Rng};
use tokio::signal::ctrl_c;
use tokio::task::JoinSet;
use tracing::{debug, error, info};

use crate::commands::{ProviderArg, BUCKET_NAME};
use crate::data::Document;
use crate::providers::chroma::ChromaProvider;
use crate::providers::topk_py::TopkPyProvider;
use crate::providers::topk_rs::TopkRsProvider;
use crate::providers::tpuf_py::TpufPyProvider;
use crate::providers::{ProviderLike, Query};
use crate::s3::pull_dataset;
use crate::telemetry::metrics::{export_metrics, read_snapshot};

#[derive(Parser, Debug, Clone)]
pub struct QueryArgs {
    #[arg(long, help = "Target collection")]
    pub(crate) collection: String,

    #[arg(short, long, help = "Name of the dataset to query")]
    pub(crate) dataset: Option<String>,

    #[arg(short, long, help = "Input file to query")]
    pub(crate) input: Option<String>,

    #[arg(short, long, help = "Target collection")]
    pub(crate) provider: ProviderArg,

    #[arg(short, long, help = "Number of concurrent queries")]
    pub(crate) concurrency: usize,

    #[arg(short, long, help = "Timeout in seconds")]
    pub(crate) timeout: u64,

    #[arg(long, help = "Numeric filter")]
    pub(crate) int_filter: Option<u32>,

    #[arg(long, help = "Keyword filter")]
    pub(crate) keyword_filter: Option<String>,

    #[arg(long, help = "Top K")]
    pub(crate) top_k: u32,
}

pub async fn run(args: QueryArgs) -> anyhow::Result<()> {
    // Generate ingest ID
    let query_id = uuid::Uuid::new_v4()
        .to_string()
        .chars()
        .take(8)
        .collect::<String>();

    info!("Starting query: {:?} with ID: {}", args, query_id);

    // Determine input path
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
        ProviderArg::Chroma => ChromaProvider::new().await?,
    };

    // Ping provider
    // First ping to ensure the provider is ready
    provider.ping(args.collection.clone()).await?;
    // Then measure
    for _ in 0..3 {
        let latency = provider.ping(args.collection.clone()).await?;
        info!("Ping latency: {:?}", latency);
    }

    let queries = spawn_query_generator(dataset_path)?;

    // Spawn writers
    let workers = spawn_workers(
        provider.clone(),
        args.collection.clone(),
        queries.clone(),
        args.top_k,
        args.int_filter,
        args.keyword_filter,
        args.concurrency,
    );

    // Spawn metrics reporter
    let stats = spawn_metrics_reporter();

    // Build metrics metadata
    let metadata = vec![
        KeyValue::new("provider".into(), format!("{:?}", args.provider)),
        KeyValue::new("collection".into(), args.collection),
        KeyValue::new("concurrency".into(), args.concurrency.to_string()),
    ];

    // Set timeout
    let timeout = tokio::time::sleep(Duration::from_secs(args.timeout));

    let start = Instant::now();
    tokio::select! {
        res = workers => {
            if let Err(error) = res {
                error!(?error, "Writer tasks exited");
            }
        }
        res = stats => {
            error!(?res, "Metrics reporter exited");
        }
        _ = timeout => {
            info!("Queries completed in {:.2}s", start.elapsed().as_secs_f64());

            // Close provider
            if let Err(error) = provider.close().await {
                error!(?error, "Failed to close provider");
            }

            // Export metrics
            if let Err(error) = export_metrics(BUCKET_NAME, metadata, &query_id).await {
                error!(?error, "Failed to export metrics");
            }

            std::process::exit(0);
        }
        _ = ctrl_c() => {
            info!("Queries interrupted after {:.2}s", start.elapsed().as_secs_f64());

            // Close provider
            if let Err(error) = provider.close().await {
                error!(?error, "Failed to close provider");
            }

            // Export metrics
            if let Err(error) = export_metrics(BUCKET_NAME, metadata, &query_id).await {
                error!(?error, "Failed to export metrics");
            }

            std::process::exit(128 + 2);
        }
    }

    Ok(())
}

// Spawn query generator task
fn spawn_query_generator(queries: PathBuf) -> anyhow::Result<Receiver<PqQuery>> {
    let (tx, rx) = async_channel::unbounded::<PqQuery>();

    let file = File::open(&queries).context("Failed to open queries file")?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .context("Failed to create ParquetRecordBatchReaderBuilder")?;
    let reader = builder
        .build()
        .context("Failed to build ParquetRecordBatchReader")?;
    let batches = reader
        .collect::<Result<Vec<RecordBatch>, _>>()
        .context("Failed to read record batches from queries file")?;
    let queries: Vec<PqQuery> = batches
        .iter()
        .map(|batch| decode_vectors_fast(batch))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();

    std::thread::spawn(move || loop {
        let random_query = queries
            .choose(&mut thread_rng())
            .expect("Failed to choose query")
            .clone();

        if tx.send_blocking(random_query).is_err() {
            error!("Failed to send query to channel");
            break;
        }
    });

    Ok(rx)
}

#[allow(dead_code)]
#[derive(serde::Deserialize, Debug, Clone)]
struct PqQuery {
    text: String,
    dense: Vec<f32>,
    sparse_keys: Vec<u32>,
    sparse_values: Vec<f32>,
    recall_dense_100: HashMap</*int*/ u32, HashMap</*keyword*/ String, /*doc IDs*/ Vec<i64>>>,
}

impl PqQuery {
    fn recall(
        &self,
        top_k: u32,
        int_filter: Option<u32>,
        keyword_filter: Option<String>,
    ) -> anyhow::Result<Vec<u32>> {
        assert!(top_k <= 100, "top_k must be less than or equal to 100");

        let int_filter = int_filter.unwrap_or(10000);
        let keyword_filter = keyword_filter.unwrap_or("10000".to_string());

        let doc_ids = self
            .recall_dense_100
            .get(&int_filter)
            .expect("int_filter not found")
            .get(&keyword_filter)
            .expect("keyword_filter not found")
            .clone()
            .into_iter()
            .filter(|x| x.is_positive())
            .map(|x| x as u32)
            .collect::<Vec<_>>();

        Ok(doc_ids)
    }
}

fn decode_vectors_fast(batch: &RecordBatch) -> anyhow::Result<Vec<PqQuery>> {
    use arrow::json::LineDelimitedWriter;

    // Convert RecordBatch to JSON line-delimited format
    let mut json_buffer = Vec::new();
    {
        let mut writer = LineDelimitedWriter::new(&mut json_buffer);
        writer.write_batches(&[batch])?;
        writer.finish()?;
    }

    // Deserialize each JSON line to PqQuery
    let mut vectors = Vec::new();
    let json_str = String::from_utf8(json_buffer)?;
    for line in json_str.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let query: PqQuery = serde_json::from_str(line)?;
        vectors.push(query);
    }

    Ok(vectors)
}

// Spawn worker tasks
async fn spawn_workers(
    provider: impl ProviderLike + Send + Sync + Clone + 'static,
    collection: String,
    queries: Receiver<PqQuery>,
    top_k: u32,
    int_filter: Option<u32>,
    keyword_filter: Option<String>,
    concurrency: usize,
) -> anyhow::Result<()> {
    // Spawn worker tasks
    let mut workers = JoinSet::new();

    for _ in 0..concurrency {
        let collection = collection.clone();
        let queries = queries.clone();
        let provider = provider.clone();
        let int_filter = int_filter.clone();
        let keyword_filter = keyword_filter.clone();

        workers.spawn(async move {
            while let Ok(vector) = queries.recv().await {
                // Extract dense vector before moving
                let dense_vector = vector.dense.clone();

                // Build query
                let query = Query {
                    vector: dense_vector,
                    top_k,
                    int_filter: int_filter.clone(),
                    keyword_filter: keyword_filter.clone(),
                };

                loop {
                    let s = Instant::now();
                    counter!("bench.query.requests").increment(1);
                    match provider.query(collection.clone(), query.clone()).await {
                        Ok(res) => {
                            counter!("bench.query.oks").increment(1);
                            let latency = s.elapsed();
                            histogram!("bench.query.latency_ms").record(latency.as_millis() as f64);
                            debug!(?res, ?latency, k = res.len(), "Returned documents");

                            let recall = calculate_recall(
                                res,
                                vector,
                                top_k,
                                int_filter,
                                keyword_filter.clone(),
                            )
                            .expect("failed to calculate recall");
                            histogram!("bench.query.recall").record(recall as f64);

                            break;
                        }
                        Err(error) => {
                            let latency = s.elapsed();
                            info!("Err. latency={:?}", latency);
                            counter!("bench.query.errors").increment(1);
                            histogram!("bench.query.latency_ms").record(latency.as_millis() as f64);
                            error!(?error, ?latency, "Failed to query documents");
                            // Sleep
                            let jitter = thread_rng().gen_range(10..100);
                            tokio::time::sleep(Duration::from_millis(jitter)).await;
                        }
                    }
                }
            }
        });
    }

    while let Some(res) = workers.join_next().await {
        match res {
            Ok(_) => continue,
            Err(error) => {
                error!(?error, "Worker task panicked");
                break;
            }
        }
    }

    Ok(())
}

// metrics reporter task
fn spawn_metrics_reporter() -> tokio::task::JoinHandle<()> {
    tokio::task::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(1));
        loop {
            ticker.tick().await;
            let metrics = read_snapshot().await;
            // let get_count = |name: &str| metrics.get(name).map(|m| m.count()).unwrap_or_default();
            let get_sum = |name: &str| metrics.get(name).map(|m| m.sum()).unwrap_or_default();
            let get_avg = |name: &str| metrics.get(name).map(|m| m.mean()).unwrap_or_default();
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

            let requests = get_sum("bench.query.requests");
            let errors = get_sum("bench.query.errors");

            let availability = (1.0 - errors as f64 / requests as f64) * 100.0;
            let avg_latency = metrics
                .get("bench.query.latency_ms")
                .map(|m| m.mean())
                .unwrap_or_default();
            println!(
                "STATS: Availability: {}, Throughput: {}, Latency: {}, {}, {}, {}, Recall: {}",
                // Availability
                match availability {
                    _ if availability == 100.0 => format!("100%").green().bold(),
                    _ if availability > 99.0 => format!("{:.2}%", availability).yellow().bold(),
                    _ if availability.is_nan() => format!("...").bold(),
                    _ => format!("{:.2}%", availability).red().bold(),
                },
                // Throughput
                format!("{:.2} queries/s", get_rate("bench.query.requests"))
                    .yellow()
                    .bold(),
                // Latency
                format!("avg={:.2}ms", avg_latency).yellow().bold(),
                format!("p50={:.2}ms", get_quantile("bench.query.latency_ms", 0.5))
                    .blue()
                    .bold(),
                format!("p90={:.2}ms", get_quantile("bench.query.latency_ms", 0.9))
                    .magenta()
                    .bold(),
                format!("p99={:.2}ms", get_quantile("bench.query.latency_ms", 0.99))
                    .cyan()
                    .bold(),
                // Recall
                format!("avg={:.2}", get_avg("bench.query.recall"))
                    .yellow()
                    .bold(),
            );
        }
    })
}

fn calculate_recall(
    res: Vec<Document>,
    vector: PqQuery,
    top_k: u32,
    int_filter: Option<u32>,
    keyword_filter: Option<String>,
) -> anyhow::Result<f32> {
    // Get expected doc IDs for recall calculation
    let actual_doc_ids = res
        .iter()
        .map(|x| {
            x.get("_id")
                .or_else(|| x.get("id"))
                .expect("missing _id or id")
                .as_string()
                .expect("_id is not a string")
                .to_string()
                .parse::<u32>()
                .expect("_id is not a u32")
        })
        .collect::<HashSet<u32>>();

    let expected_doc_ids = vector
        .recall(top_k, int_filter, keyword_filter.clone())
        .expect("failed to calculate recall")
        .into_iter()
        .take(top_k as usize)
        .collect::<HashSet<u32>>();

    let found_doc_ids = actual_doc_ids.intersection(&expected_doc_ids).count();

    Ok(found_doc_ids as f32 / expected_doc_ids.len() as f32)
}
