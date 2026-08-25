use std::collections::HashMap;

use elasticsearch::auth::Credentials;
use elasticsearch::http::transport::{SingleNodeConnectionPool, TransportBuilder};
use elasticsearch::indices::IndicesGetMappingParts;
use elasticsearch::{Elasticsearch, OpenPointInTimeParts, SearchParts};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value as JsonValue};

use topk_rs::proto::v1::control::FieldSpec;

use crate::import::error::Error;
use crate::import::source::codec::spec as codec;
use crate::import::spec::{Field, Target, Type};
use crate::import::ID;

use super::{Chunk, Records, Table};

const PAGE: i64 = 1000;
// Renewed per page; also how long a stopped run's cursor stays resumable (ES's ceiling).
const KEEP_ALIVE: &str = "24h";

/// The resume cursor: a page position is only meaningful inside its PIT.
#[derive(Serialize, Deserialize)]
struct Cursor {
    pit: String,
    sort: Vec<JsonValue>,
}

pub struct Es {
    client: Elasticsearch,
}

impl Es {
    pub fn new(mut endpoint: url::Url) -> Result<Es, Error> {
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
        })
    }

    pub async fn catalog(&self) -> Result<Vec<Table>, Error> {
        // Bare GET /_mapping: naming indices in the URL overflows the 4KB request
        // line. Deserialized per index so one unreadable mapping skips one index.
        let mappings: HashMap<String, JsonValue> = self
            .client
            .indices()
            .get_mapping(IndicesGetMappingParts::None)
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

        Ok(indices
            .into_iter()
            .filter_map(|(from, raw)| {
                let fields = match serde_json::from_value::<topk_es::api::IndexMapping>(raw)
                    .map_err(|e| e.to_string())
                    .and_then(|index| {
                        HashMap::<String, FieldSpec>::try_from(index).map_err(|e| e.to_string())
                    }) {
                    Ok(fields) => fields,
                    Err(error) => {
                        eprintln!("skipping index {from:?}: unreadable mapping: {error}");
                        return None;
                    }
                };
                let columns = std::iter::once((
                    ID.to_string(),
                    Field {
                        ty: Type::Text,
                        ..Default::default()
                    },
                ))
                .chain(
                    fields
                        .iter()
                        .map(|(name, spec)| (name.clone(), codec::field(spec))),
                )
                .collect();
                Some(Table {
                    collection_hint: Some(from.clone()),
                    from,
                    columns,
                    primary_key: None,
                })
            })
            .collect())
    }

    pub async fn stream(&self, target: &Target, after: Option<&str>) -> Result<Records, Error> {
        let client = self.client.clone();
        let index = target.from.clone();
        let limit = target.limit;
        let query = match target.filter.as_deref() {
            Some(f) => serde_json::from_str(f).map_err(|e| {
                Error::InvalidArgument(format!(
                    "filter {f:?} is not JSON — elasticsearch filters are query DSL objects, \
                     e.g. '{{\"range\": {{\"year\": {{\"gt\": 2000}}}}}}' ({e})"
                ))
            })?,
            None => json!({ "match_all": {} }),
        };
        let resume: Option<Cursor> = after.map(serde_json::from_str).transpose()?;

        let stream = async_stream::stream! {
            let size = limit.map(|l| (l as i64).min(PAGE)).unwrap_or(PAGE);
            let resumed = resume.is_some();
            let (mut pit, mut cursor) = match resume {
                Some(cursor) => (cursor.pit, Some(cursor.sort)),
                None => match open_pit(&client, &index).await {
                    Ok(pit) => (pit, None),
                    Err(e) => {
                        yield Chunk { rows: vec![Err(e)], mark: None };
                        return;
                    }
                },
            };
            let mut yielded: u64 = 0;

            loop {
                let mut body = json!({
                    "size": size,
                    "query": query.clone(),
                    "pit": { "id": pit, "keep_alive": KEEP_ALIVE },
                    "sort": [{ "_shard_doc": "asc" }],
                });
                if let Some(after) = cursor.take() {
                    body["search_after"] = JsonValue::Array(after);
                }
                let page: Page = match fetch_page(&client, body).await {
                    Ok(page) => page,
                    // A PIT ES no longer knows: 404 expired, 400 unparseable.
                    Err(Error::Es(e)) if resumed && yielded == 0
                        && e.status_code().is_some_and(|s| s.is_client_error()) =>
                    {
                        yield Chunk { rows: vec![Err(Error::Expired)], mark: None };
                        return;
                    }
                    Err(e) => {
                        yield Chunk { rows: vec![Err(e)], mark: None };
                        return;
                    }
                };
                if let Some(id) = page.pit_id {
                    pit = id;
                }

                let full = page.hits.hits.len() as i64 == size;
                let remaining = limit.map(|l| (l - yielded) as usize).unwrap_or(usize::MAX);
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
                    rows.push(row.map_err(|e| Error::Row(Box::new(e.into()))));
                }
                let mark = cursor.as_ref().and_then(|sort| {
                    serde_json::to_string(&Cursor { pit: pit.clone(), sort: sort.clone() }).ok()
                });
                yield Chunk { rows, mark };
                if !full || limit.is_some_and(|l| yielded >= l) {
                    break;
                }
            }
            close_pit(&client, &pit).await;
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
    let opened: Pit = client
        .open_point_in_time(OpenPointInTimeParts::Index(&[index]))
        .keep_alive(KEEP_ALIVE)
        .send()
        .await?
        .error_for_status_code()?
        .json()
        .await?;
    Ok(opened.id)
}

async fn fetch_page(client: &Elasticsearch, body: JsonValue) -> Result<Page, Error> {
    Ok(client
        .search(SearchParts::None)
        .body(body)
        .send()
        .await?
        .error_for_status_code()?
        .json()
        .await?)
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
    sort: Vec<JsonValue>,
}
