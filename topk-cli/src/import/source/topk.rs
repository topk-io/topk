use std::fmt;
use std::str::FromStr;

use topk_rs::proto::v1::data::Value;
use topk_rs::query::{field, filter, SortOrder};

use crate::client::{DataClient, ManagementClient};
use crate::config::Config;
use crate::import::error::Error;
use crate::import::source::Record;
use crate::import::spec::{Field, Target};
use crate::import::ID;
use crate::management::proto::GetCollectionRequest;
use crate::ProjectId;

use super::{Chunk, ChunkStream, Cursor as Checkpoint, Table};

/// A topk source has no filter language; the type has no values.
pub enum Filter {}

impl FromStr for Filter {
    type Err = Error;

    fn from_str(_: &str) -> Result<Filter, Error> {
        Err(Error::InvalidArgument(
            "--filter is not supported for topk:// sources".into(),
        ))
    }
}

/// The last `_id` read; the default is before every id.
#[derive(Default)]
pub struct Cursor(String);

impl TryFrom<Checkpoint> for Cursor {
    type Error = Error;

    fn try_from(cursor: Checkpoint) -> Result<Cursor, Error> {
        Ok(Cursor(cursor.key()?))
    }
}

#[derive(Clone)]
pub struct Uri {
    pub collection: String,
    pub project_id: Option<ProjectId>,
}

impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.project_id {
            Some(project_id) => write!(f, "topk://{project_id}/{}", self.collection),
            None => write!(f, "topk://{}", self.collection),
        }
    }
}

impl fmt::Debug for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Topk")
            .field("collection", &self.collection)
            .field("project_id", &self.project_id)
            .finish()
    }
}

#[derive(Clone)]
pub struct Topk {
    client: DataClient,
    pub table: Table,
}

impl Topk {
    pub async fn connect(uri: &Uri, config: &Config) -> Result<Topk, Error> {
        let project_id = uri
            .project_id
            .clone()
            .ok_or_else(|| Error::InvalidArgument("topk:// sources require a project".into()))?;
        let collection = ManagementClient::new(config.clone())?
            .collections
            .get_collection(GetCollectionRequest {
                project_id: project_id.to_string(),
                name: uri.collection.clone(),
            })
            .await?
            .into_inner()
            .collection
            .ok_or_else(|| {
                Error::InvalidArgument("management returned no source collection".into())
            })?;
        if collection.region.is_empty() {
            return Err(Error::InvalidArgument(
                "source collection has no region".into(),
            ));
        }
        let client = DataClient::new(config, project_id, &collection.region)?;
        let table = Table {
            columns: std::iter::once(Ok((ID.to_string(), Field::default())))
                .chain(
                    collection
                        .schema
                        .iter()
                        .map(|(name, spec)| Ok((name.clone(), Field::from(spec)))),
                )
                .collect::<Result<_, Error>>()?,
            collection_hint: Some(collection.name.clone()),
            from: collection.name.clone(),
            primary_key: Some(ID.to_string()),
        };
        Ok(Topk { client, table })
    }

    /// Keyset pages ordered by `_id`, so a page boundary is a resume point.
    /// `fetch` and not `select`: a select of an indexed vector is refused by the
    /// server, and returns the index's quantized copy where it is not.
    pub fn scan(&self, target: &Target, after: Option<Checkpoint>) -> Result<ChunkStream, Error> {
        let _ = target
            .filter
            .as_deref()
            .map(str::parse::<Filter>)
            .transpose()?;
        let Cursor(mut cursor) = after.map(Cursor::try_from).transpose()?.unwrap_or_default();
        let collection = self.client.collection(&target.from);
        // 10k is the server's cap on a sorted `limit`, and the resume granularity.
        let size = target.limit.map_or(10_000, |limit| limit.min(10_000));

        let stream = async_stream::try_stream! {
            loop {
                let documents = collection
                    .query(
                        filter(field(ID).gt(cursor.clone()))
                            .sort((field(ID), SortOrder::Asc))
                            .limit(size)
                            .fetch(["*"]),
                        None,
                        None,
                    )
                    .await?;
                let Some(last) = documents
                    .last()
                    .and_then(|document| document.fields.get(ID))
                    .and_then(Value::as_string)
                else {
                    break;
                };
                cursor = last.to_string();
                let full = documents.len() as u64 == size;
                yield Chunk {
                    rows: documents
                        .into_iter()
                        .map(|document| Ok(document.fields.into_iter().collect::<Record>()))
                        .collect(),
                    cursor: Some(Checkpoint::Key(cursor.clone())),
                };
                if !full {
                    break;
                }
            }
        };
        Ok(Box::pin(stream))
    }
}
