use std::time::{Duration, Instant};

use async_channel::{Receiver, Sender};
use clap::Parser;
use colored::Colorize;
use metrics::{counter, histogram};
use parquet::file::metadata::KeyValue;
use rand::prelude::Distribution;
use rand::{thread_rng, Rng};
use rand_distr::Uniform;
use tokio::signal::ctrl_c;
use tokio::task::JoinSet;
use tracing::{debug, error, info};

use crate::commands::{ProviderArg, BUCKET_NAME};
use crate::providers::topk_py::TopkPyProvider;
use crate::providers::topk_rs::TopkRsProvider;
use crate::providers::tpuf_py::TpufPyProvider;
use crate::providers::{ProviderLike, Query};
use crate::telemetry::metrics::{export_metrics, read_snapshot};

#[derive(Parser, Debug, Clone)]
pub struct QueryArgs {
    #[arg(long, help = "Target collection")]
    pub(crate) collection: String,

    #[arg(short, long, help = "Target collection")]
    pub(crate) provider: ProviderArg,

    #[arg(short, long, help = "Number of concurrent queries")]
    pub(crate) concurrency: usize,

    #[arg(short, long, help = "Number of queries to run")]
    pub(crate) queries: usize,

    #[arg(short, long, help = "Numeric filter")]
    pub(crate) num_filter: Option<u32>,

    #[arg(short, long, help = "Keyword filter")]
    pub(crate) keyword_filter: Option<String>,

    #[arg(short, long, help = "Top K")]
    pub(crate) top_k: usize,
}

pub async fn run(args: QueryArgs) -> anyhow::Result<()> {
    // Generate ingest ID
    let query_id = uuid::Uuid::new_v4()
        .to_string()
        .chars()
        .take(8)
        .collect::<String>();

    info!("Starting query: {:?} with ID: {}", args, query_id);

    // Create provider
    let provider = match args.provider {
        ProviderArg::TopkRs => TopkRsProvider::new(args.collection.clone()).await?,
        ProviderArg::TopkPy => TopkPyProvider::new(args.collection.clone()).await?,
        ProviderArg::TpufPy => TpufPyProvider::new(args.collection.clone()).await?,
    };

    // Ping provider
    // First ping to ensure the provider is ready
    provider.ping().await?;
    // Then measure
    for _ in 0..3 {
        let latency = provider.ping().await?;
        info!("Ping latency: {:?}", latency);
    }

    let (tx, rx) = async_channel::bounded(args.queries);

    // Spawn query generator
    std::thread::spawn(move || {
        let result = spawn_query_generator(tx, args.queries, 768);
        if let Err(error) = result {
            error!(?error, "Query generator task failed");
        }
    });

    // Spawn writers
    let workers = spawn_workers(
        provider.clone(),
        rx,
        args.top_k,
        args.num_filter,
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
        KeyValue::new("queries".into(), args.queries.to_string()),
    ];

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
        _ = ctrl_c() => {
            provider.close().await?;

            // export (partial) metrics
            export_metrics(BUCKET_NAME, metadata, &query_id).await?;

            std::process::exit(128 + 2);
        }
    }

    let duration = start.elapsed();
    info!("Ingest completed in {:.2}s", duration.as_secs_f64());

    export_metrics(BUCKET_NAME, metadata, &query_id).await?;

    Ok(())
}

// Spawn query generator task
fn spawn_query_generator(tx: Sender<Vec<f32>>, queries: usize, dim: usize) -> anyhow::Result<()> {
    let mut rng = thread_rng();

    for _ in 0..queries {
        let vector = random_uniform_vector(&mut rng, dim);

        if tx.send_blocking(vector).is_err() {
            anyhow::bail!("Failed to send query to channel");
        }
    }

    Ok(())
}

fn random_uniform_vector<R: Rng>(rng: &mut R, dim: usize) -> Vec<f32> {
    let distr = Uniform::<f32>::new(0.0, 1.0);
    distr.sample_iter(rng).take(dim).collect()
}

// Spawn worker tasks
async fn spawn_workers(
    provider: impl ProviderLike + Send + Sync + Clone + 'static,
    rx: Receiver<Vec<f32>>,
    top_k: usize,
    num_filter: Option<u32>,
    keyword_filter: Option<String>,
    concurrency: usize,
) -> anyhow::Result<()> {
    // Spawn worker tasks
    let mut workers = JoinSet::new();

    for _ in 0..concurrency {
        let rx = rx.clone();
        let provider = provider.clone();
        let keyword_filter = keyword_filter.clone();

        workers.spawn(async move {
            while let Ok(vector) = rx.recv().await {
                loop {
                    let s = Instant::now();
                    counter!("bench.query.requests").increment(1);

                    let query = Query {
                        vector: vector.clone(),
                        top_k,
                        numeric_selectivity: num_filter,
                        categorical_selectivity: keyword_filter.clone(),
                    };

                    match provider.query(query).await {
                        Ok(res) => {
                            counter!("bench.query.oks").increment(1);
                            let latency = s.elapsed();
                            histogram!("bench.query.latency_ms").record(latency.as_millis() as f64);
                            debug!(?res, ?latency, k = res.len(), "Returned documents");
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

            let requests = get_count("bench.query.requests");
            let errors = get_count("bench.query.errors");

            let availability = (1.0 - errors as f64 / requests as f64) * 100.0;
            let avg_latency = metrics
                .get("bench.query.latency_ms")
                .map(|m| m.mean())
                .unwrap_or_default();
            println!(
                "STATS: Availability: {}, Throughput: {}, Latency: {}, {}, {}, {}",
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
            );
        }
    })
}
