use arrow::datatypes::DataType;
use arrow::datatypes::Field;
use arrow::datatypes::Schema;
use arrow_array::ArrayRef;
use arrow_array::Float64Array;
use arrow_array::RecordBatch;
use arrow_array::StringArray;
use arrow_array::TimestampMicrosecondArray;
use aws_sdk_s3::primitives::ByteStream;
use chrono::Utc;
use metrics::SharedString;
use metrics::Unit;
use metrics_util::debugging::DebugValue;
use metrics_util::debugging::DebuggingRecorder;
use metrics_util::debugging::Snapshotter;
use metrics_util::CompositeKey;
use once_cell::sync::Lazy;
use parquet::arrow::ArrowWriter;
use parquet::file::metadata::KeyValue;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::info;

use crate::s3::new_client;

/// Snapshot interval in milliseconds.
const SNAPSHOT_INTERVAL: Duration = Duration::from_millis(100);

/// In-memory metrics store.
static METRIC_COLLECTORS: Lazy<Arc<RwLock<HashMap<String, MetricCollector>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

static RAW_METRICS: Lazy<
    Arc<
        RwLock<
            Vec<(
                chrono::DateTime<Utc>,
                CompositeKey,
                Option<Unit>,
                Option<SharedString>,
                DebugValue,
            )>,
        >,
    >,
> = Lazy::new(|| Arc::new(RwLock::new(Vec::new())));

/// Install the metrics recorder and start the snapshot loop.
pub fn install() -> anyhow::Result<()> {
    let recorder = DebuggingRecorder::new();
    let snapshotter = recorder.snapshotter();

    // Install the recorder globally.
    metrics::set_global_recorder(recorder)?;

    // Start `snapshot` loop
    spawn_snapshot_loop(snapshotter);

    Ok(())
}

/// Read last snapshot of in-memory metrics.
pub async fn read_snapshot() -> HashMap<String, MetricCollector> {
    METRIC_COLLECTORS.read().await.clone()
}

pub async fn export_metrics(
    bucket: &str,
    metadata: Vec<KeyValue>,
    trace_id: &str,
) -> anyhow::Result<()> {
    // Create S3 client
    let s3 = new_client()?;
    let now = chrono::Utc::now().format("%Y-%m-%d-%H-%M-%S").to_string();

    // Open local metrics file
    let mut tmpfile = tempfile::NamedTempFile::new()?;
    let file = tmpfile.as_file_mut();

    // Export metrics to a .parquet file
    write_parquet(&file, metadata).await?;

    // Read metrics file into a buffer
    let mut buffer = Vec::new();
    file.rewind()?;
    file.read_to_end(&mut buffer)?;

    // Upload metrics to S3
    s3.put_object()
        .bucket(bucket)
        .key(format!("results/topk-bench-{now}-{trace_id}.parquet"))
        .body(ByteStream::from(buffer))
        .send()
        .await?;

    info!("Metrics exported to s3://{bucket}/results/topk-bench-{now}-{trace_id}.parquet");

    Ok(())
}

/// Export metrics to a .parquet file
async fn write_parquet(file: &File, metadata: Vec<KeyValue>) -> anyhow::Result<()> {
    let schema = Arc::new(Schema::new(vec![
        Field::new(
            "timestamp",
            DataType::Timestamp(arrow_schema::TimeUnit::Microsecond, None),
            false,
        ),
        Field::new("metric", DataType::Utf8, false),
        Field::new("kind", DataType::Utf8, false), // e.g. counter/gauge/histogram
        Field::new("value", DataType::Float64, false),
        Field::new("quantile", DataType::Float64, true), // for histogram quantiles
    ]));

    let snapshot = RAW_METRICS.read().await;

    let mut timestamps = Vec::new();
    let mut metrics = Vec::new();
    let mut kinds = Vec::new();
    let mut values = Vec::new();
    let mut quantiles = Vec::new();

    for (timestamp, key, _, _, value) in snapshot.iter() {
        let ts = timestamp.timestamp_micros();
        let metric_name = key.key().name().to_string();

        match value {
            DebugValue::Counter(v) => {
                timestamps.push(ts);
                metrics.push(metric_name.clone());
                kinds.push("counter".to_string());
                values.push(*v as f64);
                quantiles.push(None);
            }
            DebugValue::Gauge(v) => {
                timestamps.push(ts);
                metrics.push(metric_name.clone());
                kinds.push("gauge".to_string());
                values.push(v.into_inner());
                quantiles.push(None);
            }
            DebugValue::Histogram(hist) => {
                // Skip empty histograms
                if hist.is_empty() {
                    continue;
                }

                // Collect and sort values for proper quantile calculation
                let mut sorted_values: Vec<f64> = hist.iter().map(|v| v.into_inner()).collect();
                sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

                // Compute a few representative quantiles
                let qs = [0.5, 0.9, 0.95, 0.99];
                let len = sorted_values.len();
                for q in qs {
                    // Calculate quantile index: quantile * (len - 1)
                    // This gives us the proper percentile position
                    let index = ((q * (len - 1) as f64).round() as usize).min(len - 1);
                    let value = sorted_values[index];

                    timestamps.push(ts);
                    metrics.push(metric_name.clone());
                    kinds.push("histogram".to_string());
                    values.push(value);
                    quantiles.push(Some(q));
                }
            }
        }
    }

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(TimestampMicrosecondArray::from(timestamps)) as ArrayRef,
            Arc::new(StringArray::from(metrics)) as ArrayRef,
            Arc::new(StringArray::from(kinds)) as ArrayRef,
            Arc::new(Float64Array::from(values)) as ArrayRef,
            Arc::new(Float64Array::from(quantiles)) as ArrayRef,
        ],
    )?;

    let mut writer = ArrowWriter::try_new(file.try_clone()?, schema, None)?;

    for kv in metadata.into_iter() {
        writer.append_key_value_metadata(kv);
    }

    // Write batch to file
    writer.write(&batch)?;
    writer.close()?;

    Ok(())
}

fn spawn_snapshot_loop(snapshotter: Snapshotter) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            // Snapshot every `SNAPSHOT_INTERVAL`
            tokio::time::sleep(SNAPSHOT_INTERVAL).await;

            // Acquire the lock
            let mut metrics = METRIC_COLLECTORS.write().await;
            let mut raw_metrics = RAW_METRICS.write().await;

            // Record metrics from the snapshot
            for (key, unit, desc, value) in snapshotter.snapshot().into_vec() {
                // Snapshot the metric for progress reporting
                metrics
                    .entry(key.key().name().to_string())
                    .or_insert(MetricCollector::default())
                    .record(&value);

                // Record the raw metric
                raw_metrics.push((Utc::now(), key, unit, desc, value));
            }
        }
    })
}

#[derive(Debug, Clone)]
pub struct MetricCollector {
    created_at: Instant,
    last: f64,
    count: u64,
    sum: f64,
    min: f64,
    max: f64,
    histogram: hdrhistogram::Histogram<u64>,
    // Simple rate interpolation: track last update
    last_sum: f64,
    last_update_time: Option<Instant>,
}

impl Default for MetricCollector {
    fn default() -> Self {
        Self {
            created_at: Instant::now(),
            last: 0.0,
            count: 0,
            sum: 0.0,
            min: f64::MAX,
            max: f64::MIN,
            histogram: hdrhistogram::Histogram::new(4).unwrap(),
            last_sum: 0.0,
            last_update_time: None,
        }
    }
}

#[allow(dead_code)]
impl MetricCollector {
    pub fn age(&self) -> Duration {
        Instant::now().duration_since(self.created_at)
    }

    pub fn rate_per_sec(&self) -> f64 {
        let age = self.age();
        let age_secs = age.as_secs_f64();

        // For counters, wait at least 1 second before calculating rates to avoid inflated rates at startup
        if age_secs < 1.0 {
            return 0.0;
        }

        // Use last update rate if recent (within 10 seconds), otherwise fall back to average
        // This provides simple interpolation between batch completions
        if let Some(last_time) = self.last_update_time {
            let time_since_update = Instant::now().duration_since(last_time).as_secs_f64();
            if time_since_update < 10.0 && time_since_update > 0.0 {
                let delta = self.sum - self.last_sum;
                if delta >= 0.0 {
                    return delta / time_since_update;
                }
            }
        }

        // Fall back to average rate
        self.sum / age_secs
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn sum(&self) -> f64 {
        self.sum
    }

    pub fn min(&self) -> f64 {
        self.min
    }

    pub fn max(&self) -> f64 {
        self.max
    }

    pub fn mean(&self) -> f64 {
        self.histogram.mean() / 1_000_000.0
    }

    pub fn quantile(&self, quantile: f64) -> f64 {
        (self.histogram.value_at_quantile(quantile) as f64) / 1_000_000.0
    }

    fn record(&mut self, metric: &DebugValue) {
        match metric {
            DebugValue::Counter(value) => {
                let value = *value as f64;

                self.count += 1;
                self.last = value;
                self.last_sum = self.sum;
                self.sum = value;
                self.last_update_time = Some(Instant::now());
                
                self.min = self.min.min(value);
                self.max = self.max.max(value);
            }
            DebugValue::Gauge(value) => {
                let value = value.into_inner();

                self.count += 1;
                self.last = value;
                self.sum = value;
                self.min = self.min.min(value);
                self.max = self.max.max(value);
            }
            DebugValue::Histogram(values) => {
                for value in values {
                    let value = value.into_inner();

                    self.count += 1;
                    self.last = value;
                    self.sum = value;
                    self.min = self.min.min(value);
                    self.max = self.max.max(value);

                    self.histogram
                        .record((value * 1_000_000.0) as u64)
                        .expect("failed to record metric");
                }
            }
        };
    }
}
