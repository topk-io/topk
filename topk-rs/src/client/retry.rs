use rand::prelude::*;
use std::{future::Future, time::Duration};

// On the backend, default timeout:
// - for `Query` or `Get` is 60 seconds
// - for `Upsert` is 15 seconds
// - for Control Plane operations is 15 seconds
//
// Total timeout is set to 180s to account for additional latency.
pub const DEFAULT_TIMEOUT: u64 = 180_000; // 3 minutes

// Default retry config
pub const DEFAULT_MAX_RETRIES: usize = 3; // 3 retries
pub const DEFAULT_INIT_BACKOFF: u64 = 100; // 100 milliseconds
pub const DEFAULT_MAX_BACKOFF: u64 = 10_000; // 10 seconds
pub const DEFAULT_BASE: u32 = 2; // `Base` is the multiplier for the backoff

#[derive(Clone, Debug)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: usize,

    /// Total timeout for the retry chain
    pub timeout: Duration,

    /// Backoff configuration
    pub backoff: BackoffConfig,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_MAX_RETRIES,
            timeout: Duration::from_millis(DEFAULT_TIMEOUT),
            backoff: BackoffConfig::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BackoffConfig {
    /// Base for the backoff
    pub base: u32,

    /// Initial backoff
    pub init_backoff: Duration,

    /// Maximum backoff
    pub max_backoff: Duration,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            base: DEFAULT_BASE,
            init_backoff: Duration::from_millis(DEFAULT_INIT_BACKOFF),
            max_backoff: Duration::from_millis(DEFAULT_MAX_BACKOFF),
        }
    }
}

pub async fn call_with_retry<F, T>(
    retry_config: &RetryConfig,
    f: impl Fn() -> F,
) -> Result<T, crate::Error>
where
    F: Future<Output = Result<T, crate::Error>>,
{
    // Max backoff starts at `init_backoff` and is multiplied by `base` for each retry
    let mut next_backoff = retry_config.backoff.init_backoff;

    // Retry chain
    let retry_chain = async {
        for i in 0..retry_config.max_retries + 1 {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if !e.is_retryable() || i == retry_config.max_retries {
                        return Err(e);
                    }

                    // Generate random backoff between `init_backoff` and `next_backoff`
                    let backoff = rand::thread_rng()
                        .gen_range(retry_config.backoff.init_backoff..=next_backoff);

                    // Sleep for backoff
                    tokio::time::sleep(backoff).await;

                    // Calculate next backoff
                    next_backoff = (next_backoff * retry_config.backoff.base)
                        .min(retry_config.backoff.max_backoff);
                }
            }
        }

        unreachable!()
    };

    // Enfore total timeout
    match tokio::time::timeout(retry_config.timeout, retry_chain).await {
        Ok(result) => result,
        Err(_) => return Err(crate::Error::RetryTimeout),
    }
}
