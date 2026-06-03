use std::future::Future;
use std::sync::Arc;

use napi::bindgen_prelude::*;
use napi::tokio::sync::{mpsc, Mutex};
use napi_derive::napi;

use super::AsyncGenerator;

pub(crate) type PartitionListStreamMessage = std::result::Result<Partition, String>;

/// A partition within a collection.
#[napi(object)]
pub struct Partition {
    /// Partition name
    pub name: String,
    /// RFC3339 created_at timestamp
    pub created_at: String,
}

impl From<topk_rs::proto::v1::data::Partition> for Partition {
    fn from(partition: topk_rs::proto::v1::data::Partition) -> Self {
        Self {
            name: partition.name,
            created_at: partition.created_at,
        }
    }
}

/// Iterator for partition list responses.
#[napi(async_iterator)]
pub struct PartitionListStream {
    receiver: Arc<Mutex<mpsc::Receiver<PartitionListStreamMessage>>>,
}

impl PartitionListStream {
    pub(crate) fn new(receiver: mpsc::Receiver<PartitionListStreamMessage>) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }
}

#[napi]
impl AsyncGenerator for PartitionListStream {
    type Yield = Partition;
    type Next = ();
    type Return = ();

    fn next(
        &mut self,
        _value: Option<Self::Next>,
    ) -> impl Future<Output = Result<Option<Self::Yield>>> + Send + 'static {
        let receiver = self.receiver.clone();
        async move {
            let mut guard = receiver.lock().await;
            match guard.recv().await {
                Some(Ok(v)) => Ok(Some(v)),
                Some(Err(e)) => Err(napi::Error::from_reason(e)),
                None => Ok(None),
            }
        }
    }

    fn complete(
        &mut self,
        _value: Option<Self::Return>,
    ) -> impl Future<Output = Result<Option<Self::Yield>>> + Send + 'static {
        let receiver = self.receiver.clone();
        async move {
            receiver.lock().await.close();
            Ok(None)
        }
    }
}

#[napi]
impl PartitionListStream {
    /// Returns the next partition in the stream.
    #[napi]
    pub async unsafe fn next(&mut self) -> napi::Result<Option<Partition>> {
        let mut guard = self.receiver.lock().await;
        match guard.recv().await {
            Some(Ok(v)) => Ok(Some(v)),
            Some(Err(e)) => Err(napi::Error::from_reason(e)),
            None => Ok(None),
        }
    }
}
