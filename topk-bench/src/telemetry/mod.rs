pub mod logs;
pub mod metrics;

/// Install telemetry components:
pub fn install() -> anyhow::Result<()> {
    // Record in-memory metrics
    metrics::install()?;

    // Log progress to the console
    logs::install()?;

    Ok(())
}
