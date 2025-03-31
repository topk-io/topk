use pyo3::{marker::Ungil, Python};
use std::{future::Future, sync::Arc};

/// Runtime is a wrapper around tokio::runtime::Runtime that allows for blocking on futures.
/// It yields the GIL when blocking so that the Python interpreter can continue running.
/// See https://pyo3.rs/v0.24.0/parallelism.html for more information.
pub struct Runtime {
    runtime: Arc<tokio::runtime::Runtime>,
}

impl Runtime {
    pub fn new() -> Result<Self, std::io::Error> {
        Ok(Self {
            runtime: Arc::new(tokio::runtime::Runtime::new()?),
        })
    }

    pub fn block_on<F: Future + Send>(&self, py: Python<'_>, future: F) -> F::Output
    where
        F::Output: Ungil,
    {
        py.allow_threads(move || self.runtime.block_on(future))
    }
}
