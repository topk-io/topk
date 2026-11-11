use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

use elasticsearch::auth::Credentials;
use elasticsearch::http::response::Response;
use elasticsearch::http::transport::{SingleNodeConnectionPool, TransportBuilder};
use elasticsearch::indices::{IndicesExistsParts, IndicesGetMappingParts};
use elasticsearch::{Elasticsearch, OpenPointInTimeParts, SearchParts};
use serde::Deserialize;
use serde_json::{json, Map, Value as JsonValue};

use topk_rs::proto::v1::control::FieldSpec;

use crate::import::error::Error;
use crate::import::spec::Target;
use crate::import::ID;

use super::{Catalog, Chunk, ReadEvent, ReadStream, Table};

// Renewed per page; also how long a stopped run's cursor stays resumable (ES's ceiling).
const KEEP_ALIVE: &str = "24h";

#[derive(Clone)]
pub struct Es {
    client: Elasticsearch,
    index: String,
}

/// A query DSL object; the default matches everything.
pub struct Filter(Map<String, JsonValue>);

impl Default for Filter {
    fn default() -> Filter {
        Filter(Map::from_iter([("match_all".to_string(), json!({}))]))
    }
}

impl FromStr for Filter {
    type Err = Error;

    fn from_str(filter: &str) -> Result<Filter, Error> {
        serde_json::from_str(filter).map(Filter).map_err(|e| {
            Error::InvalidArgument(format!(
                "filter {filter:?} is not a JSON object — elasticsearch filters are query DSL \
                 objects, e.g. '{{\"range\": {{\"year\": {{\"gt\": 2000}}}}}}' ({e})"
            ))
        })
    }
}

/// A page position, meaningful only inside its PIT.
pub struct Cursor {
    pit: String,
    sort: Vec<i64>,
}

impl TryFrom<super::Cursor> for Cursor {
    type Error = Error;

    fn try_from(cursor: super::Cursor) -> Result<Cursor, Error> {
        match cursor {
            super::Cursor::Page { pit, sort } => Ok(Cursor { pit, sort }),
            other => Err(Error::InvalidArgument(format!(
                "resume cursor {other} is not a page"
            ))),
        }
    }
}

impl Es {
    pub fn new(mut endpoint: url::Url) -> Result<Es, Error> {
        let index = endpoint.path().trim_matches('/').to_string();
        endpoint.set_path("/");
        let user = endpoint.username().to_string();
        let pass = endpoint.password().map(str::to_string);
        let _ = endpoint.set_username("");
        let _ = endpoint.set_password(None);

        let credentials = if !user.is_empty() {
            Some(Credentials::Basic(user, pass.unwrap_or_default()))
        } else if let Ok(key) = std::env::var("ELASTIC_API_KEY") {
            Some(Credentials::EncodedApiKey(key))
        } else {
            std::env::var("ELASTIC_PASSWORD")
                .ok()
                .map(|password| Credentials::Basic("elastic".to_string(), password))
        };

        let mut transport = TransportBuilder::new(SingleNodeConnectionPool::new(endpoint));
        if let Some(credentials) = credentials {
            transport = transport.auth(credentials);
        }
        let transport = transport
            .build()
            .map_err(|e| Error::InvalidArgument(e.to_string()))?;

        Ok(Es {
            client: Elasticsearch::new(transport),
            index,
        })
    }

    pub async fn catalog(&self) -> Result<Catalog, Error> {
        // Bare GET /_mapping: naming indices in the URL overflows the 4KB request
        // line. Deserialized per index so one unreadable mapping skips one index.
        let index = [self.index.as_str()];
        let mappings: HashMap<String, JsonValue> = self
            .client
            .indices()
            .get_mapping(if self.index.is_empty() {
                IndicesGetMappingParts::None
            } else {
                IndicesGetMappingParts::Index(&index)
            })
            .send()
            .await?
            .error_for_status_code()?
            .json()
            .await?;
        // Sorted for deterministic specs.
        let mut indices: Vec<(String, JsonValue)> = mappings
            .into_iter()
            .filter(|(name, _)| !name.starts_with('.'))
            .collect();
        indices.sort_by(|(a, _), (b, _)| a.cmp(b));

        let tables = indices
            .into_iter()
            .filter_map(|(from, raw)| {
                let fields: BTreeMap<String, FieldSpec> =
                    match serde_json::from_value::<topk_es::api::IndexMapping>(raw)
                        .map_err(|e| e.to_string())
                        .and_then(|index| {
                            HashMap::<String, FieldSpec>::try_from(index).map_err(|e| e.to_string())
                        }) {
                        Ok(fields) => fields.into_iter().collect(),
                        Err(error) => {
                            tracing::warn!("skipping index {from:?}: unreadable mapping: {error}");
                            return None;
                        }
                    };
                let columns = std::iter::once((ID.to_string(), FieldSpec::text(false)))
                    .chain(fields)
                    .map(|(name, field)| (name, Some(field)))
                    .collect();
                Some(Table {
                    collection_hint: Some(from.clone()),
                    from,
                    columns,
                    key: Some(ID.to_string()),
                })
            })
            .collect();
        Ok(Catalog { tables })
    }

    pub async fn validate(&self, from: &str) -> Result<(), Error> {
        let response = self
            .client
            .indices()
            .exists(IndicesExistsParts::Index(&[from]))
            .send()
            .await?;
        if response.status_code().as_u16() == 404 {
            return Err(Error::InvalidArgument(format!(
                "source index {from:?} was not found"
            )));
        }
        response.error_for_status_code()?;
        Ok(())
    }

    pub fn chunks(
        &self,
        target: &Target,
        after: Option<super::Cursor>,
        consumed: u64,
    ) -> Result<ReadStream, Error> {
        let Filter(query) = target
            .filter
            .as_deref()
            .map(str::parse)
            .transpose()?
            .unwrap_or_default();
        let after = after.map(Cursor::try_from).transpose()?;
        let client = self.client.clone();
        let from = target.from.clone();
        let limit = target.limit;
        let unfiltered = target.filter.is_none();

        let stream = async_stream::stream! {
            let size = limit.map_or(1000, |l| l.min(1000)) as i64;
            let mut can_restart = after.is_some();
            let (mut pit, mut cursor) = match after {
                Some(Cursor { pit, sort }) => (pit, Some(sort)),
                None => match open_pit(&client, &from).await {
                    Ok(pit) => (pit, None),
                    Err(e) => {
                        yield Err(e);
                        return;
                    }
                },
            };
            let mut yielded = consumed;

            loop {
                let mut body = json!({
                    "size": size,
                    "query": query.clone(),
                    "pit": { "id": pit, "keep_alive": KEEP_ALIVE },
                    "sort": [{ "_shard_doc": "asc" }],
                });
                if let Some(after) = cursor.take() {
                    body["search_after"] = json!(after);
                }
                let page: Page = match fetch_page(&client, body).await {
                    Ok(page) => page,
                    // A PIT ES no longer knows: 404 expired, 400 unparseable. A 400
                    // with a filter present may be the filter's; pass it through.
                    Err(Error::EsStatus { status, .. }) if can_restart
                        && (status == 404 || (status == 400 && unfiltered)) =>
                    {
                        close_pit(&client, &pit).await;
                        match open_pit(&client, &from).await {
                            Ok(fresh) => {
                                pit = fresh;
                                cursor = None;
                                can_restart = false;
                                yielded = 0;
                                yield Ok(ReadEvent::Restarted);
                                continue;
                            }
                            Err(e) => {
                                yield Err(e);
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        close_pit(&client, &pit).await;
                        yield Err(e);
                        return;
                    }
                };
                if let Some(id) = page.pit_id {
                    pit = id;
                }

                let full = page.hits.hits.len() as i64 == size;
                let remaining = limit
                    .map(|l| (l - yielded) as usize)
                    .unwrap_or(usize::MAX);
                let mut rows = Vec::with_capacity(page.hits.hits.len());
                for hit in page.hits.hits.into_iter().take(remaining) {
                    cursor = Some(hit.sort);
                    let row = hit.source.into_iter().try_fold(
                        vec![(
                            ID.to_string(),
                            topk_rs::proto::v1::data::Value::string(hit.id),
                        )],
                        |mut row, (key, value)| {
                            row.push((key, topk_rs::proto::v1::data::Value::try_from(value)?));
                            Ok::<_, topk_rs::Error>(row)
                        },
                    );
                    yielded += 1;
                    can_restart = false;
                    rows.push(row.map_err(Error::from));
                }
                let mark = cursor.clone().map(|sort| super::Cursor::Page { pit: pit.clone(), sort });
                let done = !full || limit.is_some_and(|l| yielded >= l);
                if done {
                    close_pit(&client, &pit).await;
                }
                yield Ok(ReadEvent::Chunk(Chunk { rows, cursor: mark }));
                if done {
                    break;
                }
            }
        };
        Ok(Box::pin(stream))
    }
}

async fn close_pit(client: &Elasticsearch, pit: &str) {
    let _ = client
        .close_point_in_time()
        .body(json!({ "id": pit }))
        .send()
        .await;
}

async fn open_pit(client: &Elasticsearch, index: &str) -> Result<String, Error> {
    let opened: Pit = read(
        client
            .open_point_in_time(OpenPointInTimeParts::Index(&[index]))
            .keep_alive(KEEP_ALIVE)
            .send()
            .await?,
    )
    .await?;
    Ok(opened.id)
}

async fn fetch_page(client: &Elasticsearch, body: JsonValue) -> Result<Page, Error> {
    read(client.search(SearchParts::None).body(body).send().await?).await
}

/// `error_for_status_code` drops the body, which is where elasticsearch says
/// what it disliked.
async fn read<T: for<'de> Deserialize<'de>>(response: Response) -> Result<T, Error> {
    let status = response.status_code().as_u16();
    if status >= 400 {
        let body: JsonValue = response.json().await?;
        let reason = match body["error"]["reason"].as_str() {
            Some(reason) => reason.to_string(),
            None => body.to_string(),
        };
        return Err(Error::EsStatus { status, reason });
    }
    Ok(response.json().await?)
}

#[derive(Deserialize)]
struct Pit {
    id: String,
}

#[derive(Deserialize)]
struct Page {
    pit_id: Option<String>,
    hits: Hits,
}

#[derive(Deserialize)]
struct Hits {
    hits: Vec<Hit>,
}

#[derive(Deserialize)]
struct Hit {
    #[serde(rename = "_id", default)]
    id: String,
    #[serde(rename = "_source", default)]
    source: Map<String, JsonValue>,
    #[serde(default)]
    sort: Vec<i64>,
}
