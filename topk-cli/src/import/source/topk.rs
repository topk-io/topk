use wildmatch::WildMatch;

use topk_rs::proto::v1::data::Value;
use topk_rs::query::{field, filter, SortOrder};
use topk_rs::{Client, ClientConfig};

use crate::import::error::Error;
use crate::import::source::codec::spec as codec;
use crate::import::source::Record;
use crate::import::spec::{Field, Target};
use crate::import::ID;

use super::{Chunk, Records, Table};

/// The server's cap on a sorted query's `limit`, and the resume granularity.
const PAGE: u64 = 10_000;

pub struct Topk {
    client: Client,
    collection: String,
}

impl Topk {
    /// Reads with `TOPK_SOURCE_API_KEY`, or the key the run already uses when
    /// source and target are the same org.
    pub fn new(
        region: &str,
        host: Option<&str>,
        https: Option<bool>,
        collection: &str,
    ) -> Result<Topk, Error> {
        let api_key = std::env::var("TOPK_SOURCE_API_KEY")
            .or_else(|_| std::env::var("TOPK_API_KEY"))
            .map_err(|_| {
                Error::InvalidArgument(
                    "reading topk:// needs TOPK_SOURCE_API_KEY (or TOPK_API_KEY)".to_string(),
                )
            })?;
        let host = host
            .map(str::to_string)
            .or_else(|| std::env::var("TOPK_HOST").ok())
            .unwrap_or_else(|| "topk.io".to_string());
        let https =
            https.unwrap_or_else(|| std::env::var("TOPK_HTTPS").map_or(true, |v| v != "false"));
        Ok(Topk {
            client: Client::new(
                ClientConfig::new(api_key, region.to_string())
                    .with_host(host)
                    .with_https(https),
            ),
            collection: collection.to_string(),
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
                            .map(|(name, spec)| (name.clone(), codec::field(spec))),
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
        if target.filter.is_some() {
            return Err(Error::InvalidArgument(
                "--filter is not supported for topk:// sources".to_string(),
            ));
        }
        let mut collection = self.client.collection(&target.from);
        if let Some(partition) = &target.partition {
            collection = collection.partition(partition);
        }
        let mut cursor = after.unwrap_or_default().to_string();
        let mut remaining = target.limit;

        let stream = async_stream::stream! {
            loop {
                let size = remaining.map_or(PAGE, |left| left.min(PAGE));
                if size == 0 {
                    break;
                }
                let query = filter(field(ID).gt(cursor.clone()))
                    .sort((field(ID), SortOrder::Asc))
                    .limit(size)
                    .fetch(["*"]);
                let documents = match collection.query(query, None, None).await {
                    Ok(documents) => documents,
                    Err(e) => {
                        yield Chunk { rows: vec![Err(e.into())], mark: None };
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
                yield Chunk {
                    rows: documents
                        .into_iter()
                        .map(|document| Ok(document.fields.into_iter().collect::<Record>()))
                        .collect(),
                    mark: Some(cursor.clone()),
                };
                if !full {
                    break;
                }
            }
        };
        Ok(Box::pin(stream))
    }
}
