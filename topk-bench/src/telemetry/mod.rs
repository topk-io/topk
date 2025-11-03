use tracing::info;

pub mod logs;
pub mod metrics;

const BUCKET_NAME: &str = "jergu-test";

/// Install telemetry components:
pub fn install() -> anyhow::Result<String> {
    // Generate trace ID
    let trace_id = uuid::Uuid::new_v4()
        .to_string()
        .chars()
        .take(8)
        .collect::<String>();

    // Record in-memory metrics
    metrics::install()?;

    // Log progress to the console
    logs::install()?;

    info!("Running benchmark with trace ID: {}", trace_id);

    Ok(trace_id)
}

/// Shutdown telemetry components
pub async fn shutdown(trace_id: &str) -> anyhow::Result<()> {
    metrics::export_metrics(BUCKET_NAME, trace_id).await?;

    Ok(())
}
