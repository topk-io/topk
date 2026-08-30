use std::fmt;

use wildmatch::WildMatch;

use topk_rs::proto::v1::data::Value;
use topk_rs::query::{field, filter, SortOrder};
use topk_rs::Client;

use crate::endpoint::Endpoint;
use crate::import::error::Error;
use crate::import::source::Record;
use crate::import::spec::{Field, Target};
use crate::import::ID;

use super::{Chunk, Records, Table};

/// `topk://[<key>@]<region>/<collection>`; the key defaults to the run's own.
#[derive(Clone)]
pub struct Uri {
    pub region: String,
    pub api_key: Option<String>,
    pub collection: String,
}

impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "topk://{}/{}", self.region, self.collection)
    }
}

impl fmt::Debug for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Topk")
            .field("region", &self.region)
            .field("collection", &self.collection)
            .finish()
    }
}

#[derive(Clone)]
pub struct Topk {
    client: Client,
    collection: String,
}

impl Topk {
    /// The uri names the region and may carry its own key; the host is the run's.
    pub fn connect(uri: &Uri, endpoint: &Endpoint) -> Result<Topk, Error> {
        let client = Endpoint {
            api_key: uri.api_key.clone().or_else(|| endpoint.api_key.clone()),
            region: Some(uri.region.clone()),
            ..endpoint.clone()
        }
        .client()
        .map_err(|e| Error::InvalidArgument(e.to_string()))?;
        Ok(Topk {
            client,
            collection: uri.collection.clone(),
        })
    }

    pub async fn catalog(&self) -> Result<Vec<Table>, Error> {
        let collection = &self.collection;
        // The collection is in the uri, not in OBJECTS, so its glob is matched
        // here rather than by `discover`.
        let collections = match collection.contains(['*', '?']) || collection.is_empty() {
            true => {
                let pattern =
                    WildMatch::new(collection.is_empty().then_some("*").unwrap_or(collection));
                let all = self.client.collections().list().await?;
                all.into_iter()
                    .filter(|c| pattern.matches(&c.name))
                    .collect()
            }
            false => vec![self.client.collections().get(collection).await?],
        };
        Ok(collections
            .into_iter()
            .map(|collection| Table {
                columns: std::iter::once((ID.to_string(), Field::default()))
                    .chain(
                        collection
                            .schema
                            .iter()
                            .map(|(name, spec)| (name.clone(), Field::from(spec))),
                    )
                    .collect(),
                collection_hint: Some(collection.name.clone()),
                from: collection.name,
                primary_key: Some(ID.to_string()),
            })
            .collect())
    }

    /// Keyset pages ordered by `_id`, so a page boundary is a resume point.
    /// `fetch` and not `select`: a select of an indexed vector is refused by the
    /// server, and returns the index's quantized copy where it is not.
    pub async fn stream(&self, target: &Target, after: Option<&str>) -> Result<Records, Error> {
        let collection = self.client.collection(&target.from);
        let mut cursor = after.unwrap_or_default().to_string();
        let mut remaining = target.limit;

        let stream = async_stream::stream! {
            loop {
                // 10k is the server's cap on a sorted `limit`, and the resume granularity.
                let size = remaining.map_or(10_000, |left| left.min(10_000));
                if size == 0 {
                    break;
                }
                let documents = match collection
                    .query(
                        filter(field(ID).gt(cursor.clone()))
                            .sort((field(ID), SortOrder::Asc))
                            .limit(size)
                            .fetch(["*"]),
                        None,
                        None,
                    )
                    .await
                {
                    Ok(documents) => documents,
                    Err(e) => {
                        yield Err(e.into());
                        return;
                    }
                };
                let Some(last) = documents
                    .last()
                    .and_then(|document| document.fields.get(ID))
                    .and_then(Value::as_string)
                else {
                    break;
                };
                cursor = last.to_string();
                remaining = remaining.map(|left| left - documents.len() as u64);
                let full = documents.len() as u64 == size;
                yield Ok(Chunk {
                    rows: documents
                        .into_iter()
                        .map(|document| Ok(document.fields.into_iter().collect::<Record>()))
                        .collect(),
                    mark: Some(cursor.clone()),
                });
                if !full {
                    break;
                }
            }
        };
        Ok(Box::pin(stream))
    }
}
