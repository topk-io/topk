use rand::prelude::*;
use std::future::Future;
use tokio::time::{Duration, Instant};

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
            max_retries: super::RETRY_MAX_RETRIES,
            timeout: Duration::from_millis(super::RETRY_TIMEOUT),
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
            base: super::RETRY_BACKOFF_BASE,
            init_backoff: Duration::from_millis(super::RETRY_BACKOFF_INIT),
            max_backoff: Duration::from_millis(super::RETRY_BACKOFF_MAX),
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
    let start_time = Instant::now();
    // Max backoff starts at `init_backoff` and is multiplied by `base` for each retry
    let mut next_backoff = retry_config.backoff.init_backoff;

    // Retry chain
    let retry_chain = async {
        let mut i = 0;
        loop {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    // If error is not retryable, exit early.
                    if !e.is_retryable() {
                        return Err(e);
                    }

                    // Check retry limits based on error type
                    let should_retry = match e.retry_duration() {
                        // 1. `QueryLsnTimeout`-like errors where client is expected to retry for at least X seconds
                        Some(retry_duration) => {
                            // For duration-based retries, only check duration (not max_retries)
                            // This is intentional - QueryLsnTimeout should retry for at least 2 seconds.
                            start_time.elapsed() <= retry_duration
                        }
                        // 2. `SlowDown`-like errors where client is expected to retry N times
                        None => {
                            // For count-based retries, check max_retries
                            retry_config.max_retries > 0 && i < retry_config.max_retries - 1
                        }
                    };

                    // If retries exceeded, exit.
                    if !should_retry {
                        return Err(e);
                    }

                    next_backoff = wait_for_backoff(&retry_config.backoff, next_backoff).await;
                    i += 1;
                }
            }
        }
    };

    // Enfore total timeout
    match tokio::time::timeout(retry_config.timeout, retry_chain).await {
        Ok(result) => result,
        Err(_) => Err(crate::Error::RetryTimeout),
    }
}

/// Wait for backoff and return the next backoff.
async fn wait_for_backoff(config: &BackoffConfig, next_backoff: Duration) -> Duration {
    // Generate random backoff between `init_backoff` and `next_backoff`
    let backoff = rand::thread_rng().gen_range(config.init_backoff..=next_backoff);

    // Sleep for backoff
    tokio::time::sleep(backoff).await;

    // Calculate next backoff
    (next_backoff * config.base).min(config.max_backoff)
}

#[cfg(test)]
mod tests {
    use crate::Error;

    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    async fn simulate(
        retry_config: &RetryConfig,
        f: impl Fn(usize) -> Result<(), crate::Error>,
    ) -> (usize, Result<(), crate::Error>, Duration) {
        let f = Arc::new(f);
        let counter = Arc::new(AtomicUsize::new(0));
        let count = counter.clone();
        let start_time = Instant::now();

        let result = call_with_retry(retry_config, {
            let f = f.clone();

            move || {
                let count = count.clone();
                let f = f.clone();

                async move {
                    let previous = count.fetch_add(1, Ordering::SeqCst);

                    f(previous)
                }
            }
        })
        .await;

        let total_time = start_time.elapsed();
        (counter.load(Ordering::SeqCst), result, total_time)
    }

    #[tokio::test]
    async fn non_retryable_error() {
        let retry_config = RetryConfig::default();

        let (attempts, result, _) = simulate(&retry_config, |_| {
            Err(crate::Error::Internal("test".to_string()))
        })
        .await;

        assert!(matches!(result, Err(crate::Error::Internal(_))));
        assert_eq!(attempts, 1);
    }

    #[tokio::test]
    async fn retryable_error() {
        let retry_config = RetryConfig::default();

        let (attempts, result, _) = simulate(&retry_config, |count| match count {
            0 => Err(crate::Error::SlowDown("test".to_string())),
            _ => Ok(()),
        })
        .await;

        assert!(matches!(result, Ok(_)));
        assert_eq!(attempts, 2);
    }

    #[tokio::test]
    async fn max_retries() {
        let retry_config = RetryConfig {
            max_retries: 5,
            ..Default::default()
        };

        let (attempts, result, _) =
            simulate(&retry_config, |_| Err(Error::SlowDown("test".to_string()))).await;

        assert!(matches!(result, Err(crate::Error::SlowDown(_))));
        assert_eq!(attempts, 5);
    }

    #[tokio::test]
    async fn query_lsn_timeout() {
        let retry_config = RetryConfig {
            // Set `max_retries` to 1 explicitly, to test that `QueryLsnTimeout` does not respect `max_retries`.
            max_retries: 1,
            ..Default::default()
        };

        let start_time = Instant::now();
        let (attempts, result, _) = simulate(&retry_config, |_| {
            // Always return QueryLsnTimeout to test counter increment
            Err(Error::QueryLsnTimeout)
        })
        .await;

        // Should fail due to timeout, not max_retries.
        assert!(matches!(result, Err(crate::Error::QueryLsnTimeout)));

        // Should have made multiple attempts (more than max_retries=1)
        assert!(attempts > 1);

        // Should have taken at least 2 seconds to complete.
        assert!(start_time.elapsed() >= Duration::from_millis(2_000));
    }

    #[tokio::test]
    async fn timeout_exceeded() {
        let retry_config = RetryConfig {
            timeout: Duration::from_millis(100),
            ..Default::default()
        };

        let start_time = Instant::now();
        let (attempts, result, _) = simulate(&retry_config, |_| {
            Err(Error::SlowDown("please slow down".into()))
        })
        .await;

        assert!(matches!(result, Err(crate::Error::RetryTimeout)));
        assert!(attempts > 0);
        assert!(start_time.elapsed() >= Duration::from_millis(100));
    }

    #[tokio::test]
    async fn success_first_attempt() {
        let retry_config = RetryConfig::default();

        let (attempts, result, _) = simulate(&retry_config, |_| {
            Ok(()) // Success on first attempt
        })
        .await;

        assert!(matches!(result, Ok(_)));
        assert_eq!(attempts, 1); // Should only make 1 attempt
    }

    #[tokio::test]
    async fn backoff() {
        let retry_config = RetryConfig {
            max_retries: 5,
            backoff: BackoffConfig {
                init_backoff: Duration::from_millis(10),
                max_backoff: Duration::from_millis(1000),
                base: 2,
            },
            ..Default::default()
        };

        let backoff_times = Arc::new(std::sync::Mutex::new(Vec::new()));
        let start_time = Instant::now();

        let (attempts, result, _) = simulate(&retry_config, {
            let backoff_times = backoff_times.clone();
            move |i| {
                if i > 0 {
                    backoff_times.lock().unwrap().push(start_time.elapsed());
                }
                Err(Error::SlowDown("test".to_string()))
            }
        })
        .await;

        assert!(matches!(result, Err(crate::Error::SlowDown(_))));
        assert_eq!(attempts, 5);

        // Verify backoff times are increasing
        let backoff_times = backoff_times.lock().unwrap();
        for i in 1..backoff_times.len() {
            assert!(
                backoff_times[i] > backoff_times[i - 1],
                "Backoff should increase: {:?}",
                backoff_times
            );
        }

        // Verify we have reasonable backoff progression
        assert!(
            backoff_times.len() >= 3,
            "Should have at least 3 backoff measurements"
        );
    }

    #[tokio::test]
    async fn zero_max_retries() {
        let retry_config = RetryConfig {
            max_retries: 0,
            ..Default::default()
        };

        let (attempts, result, _) =
            simulate(&retry_config, |_| Err(Error::SlowDown("test".to_string()))).await;

        assert!(matches!(result, Err(crate::Error::SlowDown(_))));
        assert_eq!(attempts, 1); // Should fail immediately
    }
}
