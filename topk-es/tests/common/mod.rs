#![allow(dead_code)]

use elasticsearch::{
    auth::Credentials,
    http::{
        request::JsonBody,
        response::Response,
        transport::{SingleNodeConnectionPool, TransportBuilder},
        StatusCode, Url,
    },
    indices::{IndicesCreateParts, IndicesDeleteParts},
    params::{Refresh, SearchType},
    BulkOperation, BulkOperations, BulkParts, CountParts, DeleteParts, Elasticsearch, GetParts,
    GetSourceParts, IndexParts, MgetParts, MsearchParts, SearchParts,
};
use serde_json::{json, Value};
use test_context::AsyncTestContext;

mod books;
#[allow(unused_imports)]
pub use books::BooksContext;

pub struct Client {
    es: Elasticsearch,
}

impl Client {
    pub fn new() -> Self {
        let token = std::env::var("ES_TOKEN")
            .or_else(|_| std::env::var("TOPK_API_KEY"))
            .expect("ES_TOKEN or TOPK_API_KEY must be set");
        let url = std::env::var("ES_URL").unwrap_or_else(|_| "http://localhost:9200".to_string());

        let url = Url::parse(&url).unwrap();
        let conn_pool = SingleNodeConnectionPool::new(url);
        let transport = TransportBuilder::new(conn_pool)
            .auth(Credentials::EncodedApiKey(token))
            .build()
            .unwrap();

        Self {
            es: Elasticsearch::new(transport),
        }
    }

    pub fn es(&self) -> &Elasticsearch {
        &self.es
    }

    pub async fn mget(&self, parts: MgetParts<'_>, body: Value) -> JsonResponse {
        let res = self.es().mget(parts).body(body).send().await.expect("mget");
        assert!(res.status_code().is_success());
        to_json(res).await
    }

    pub async fn msearch(&self, parts: MsearchParts<'_>, lines: Vec<Value>) -> JsonResponse {
        let body: Vec<JsonBody<Value>> = lines.into_iter().map(JsonBody::new).collect();
        let res = self
            .es()
            .msearch(parts)
            .body(body)
            .send()
            .await
            .expect("msearch");
        assert!(res.status_code().is_success());
        to_json(res).await
    }
}

pub struct TestScope {
    pub client: Client,
    pub name: String,
}

pub struct TwoIndices {
    pub a: TestScope,
    pub b: TestScope,
}

impl AsyncTestContext for TestScope {
    async fn setup() -> Self {
        Self {
            client: Client::new(),
            name: format!("ddb-es-proxy-test-{}", uuid::Uuid::new_v4()),
        }
    }

    async fn teardown(self) {
        let res = self
            .client
            .es()
            .indices()
            .delete(IndicesDeleteParts::Index(&[self.name.as_str()]))
            .send()
            .await
            .expect("delete index during teardown");

        assert!(res.status_code().is_success() || res.status_code() == StatusCode::NOT_FOUND);
    }
}

impl AsyncTestContext for TwoIndices {
    async fn setup() -> Self {
        Self {
            a: TestScope::setup().await,
            b: TestScope::setup().await,
        }
    }

    async fn teardown(self) {
        self.b.teardown().await;
        self.a.teardown().await;
    }
}

impl TestScope {
    pub async fn create(&self) {
        self.create_with_body(None).await.expect("create index");
    }

    pub async fn create_with_mapping(&self, mapping: Value) {
        self.create_with_body(Some(mapping))
            .await
            .expect("create index with mapping");
    }

    pub async fn create_with_properties(&self, properties: Value) {
        self.create_with_mapping(json!({ "mappings": { "properties": properties } }))
            .await;
    }

    pub async fn create_with_body(&self, body: Option<Value>) -> TestResult<()> {
        let res = match body {
            Some(body) => {
                self.client
                    .es()
                    .indices()
                    .create(IndicesCreateParts::Index(&self.name))
                    .body(body)
                    .send()
                    .await
            }
            None => {
                self.client
                    .es()
                    .indices()
                    .create(IndicesCreateParts::Index(&self.name))
                    .send()
                    .await
            }
        }
        .expect("create index");

        into_test_result(res).await.map(|_| ())
    }

    pub async fn index_doc(&self, id: &str, body: Value) -> JsonResponse {
        let res = self
            .client
            .es()
            .index(IndexParts::IndexId(&self.name, id))
            .refresh(Refresh::WaitFor)
            .body(body)
            .send()
            .await
            .expect("index doc");
        to_json(res).await
    }

    pub async fn get_doc(&self, id: &str) -> JsonResponse {
        let res = self
            .client
            .es()
            .get(GetParts::IndexId(&self.name, id))
            .send()
            .await
            .expect("get doc");
        to_json(res).await
    }

    pub async fn get_doc_with_source(
        &self,
        id: &str,
        source: Option<&[&str]>,
        includes: Option<&[&str]>,
        excludes: Option<&[&str]>,
    ) -> JsonResponse {
        let mut req = self.client.es().get(GetParts::IndexId(&self.name, id));
        if let Some(source) = source {
            req = req._source(source);
        }
        if let Some(includes) = includes {
            req = req._source_includes(includes);
        }
        if let Some(excludes) = excludes {
            req = req._source_excludes(excludes);
        }
        to_json(req.send().await.expect("get doc")).await
    }

    pub async fn get_source_with_source(
        &self,
        id: &str,
        source: Option<&[&str]>,
        includes: Option<&[&str]>,
        excludes: Option<&[&str]>,
    ) -> JsonResponse {
        let mut req = self
            .client
            .es()
            .get_source(GetSourceParts::IndexId(&self.name, id));
        if let Some(source) = source {
            req = req._source(source);
        }
        if let Some(includes) = includes {
            req = req._source_includes(includes);
        }
        if let Some(excludes) = excludes {
            req = req._source_excludes(excludes);
        }
        to_json(req.send().await.expect("get source")).await
    }

    pub async fn bulk(&self, ops: BulkOperations) -> JsonResponse {
        let res = self
            .client
            .es()
            .bulk(BulkParts::Index(&self.name))
            .refresh(Refresh::WaitFor)
            .body(vec![ops])
            .send()
            .await
            .expect("bulk");
        to_json(res).await
    }

    pub async fn index_docs<'a>(
        &self,
        docs: impl IntoIterator<Item = (&'a str, Value)>,
    ) -> JsonResponse {
        let mut ops = BulkOperations::new();
        for (id, body) in docs {
            ops.push(BulkOperation::index(body).id(id))
                .expect("encode bulk op");
        }
        let res = self.bulk(ops).await;
        assert_eq!(
            res["errors"], false,
            "index_docs must index every doc: {res}"
        );
        res
    }

    pub async fn delete_doc(&self, id: &str) -> JsonResponse {
        let res = self
            .client
            .es()
            .delete(DeleteParts::IndexId(&self.name, id))
            .refresh(Refresh::WaitFor)
            .send()
            .await
            .expect("delete doc");
        to_json(res).await
    }

    pub async fn search(&self, body: Value) -> TestResult<SearchResponse> {
        let dfs = std::env::var("ES_DFS").is_ok() && body.get("knn").is_none();

        let index = [self.name.as_str()];
        let mut req = self
            .client
            .es()
            .search(SearchParts::Index(&index))
            .body(body);

        if dfs {
            req = req.search_type(SearchType::DfsQueryThenFetch);
        }

        let res = req.send().await.expect("search");

        into_test_result(res).await.map(SearchResponse)
    }

    pub async fn search_ids(&self, query: Value) -> Vec<String> {
        self.search_ids_with_size(query, 10).await
    }

    pub async fn search_ids_with_size(&self, query: Value, size: u64) -> Vec<String> {
        let mut ids = self
            .search(json!({ "query": query, "size": size }))
            .await
            .expect("search should succeed")
            .hit_ids();
        ids.sort();
        ids
    }

    pub async fn count(&self, query: Option<Value>) -> TestResult<u64> {
        let body = match query {
            Some(q) => json!({ "query": q }),
            None => json!({}),
        };

        let res = self
            .client
            .es()
            .count(CountParts::Index(&[&self.name]))
            .body(body)
            .send()
            .await
            .expect("count");

        into_test_result(res)
            .await
            .map(|body| body["count"].as_u64().expect("count field"))
    }
}

pub type TestResult<T> = Result<T, TestErrorResponse>;

#[derive(Debug)]
pub struct TestErrorResponse {
    status: StatusCode,
    body: Value,
}

impl TestErrorResponse {
    pub fn status_code(&self) -> StatusCode {
        self.status
    }
}

#[derive(Debug)]
pub struct SearchResponse(pub Value);

pub fn hit_ids(response: &Value) -> Vec<String> {
    response["hits"]["hits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["_id"].as_str().unwrap().to_string())
        .collect()
}

impl std::ops::Deref for SearchResponse {
    type Target = Value;

    fn deref(&self) -> &Value {
        &self.0
    }
}

impl std::fmt::Display for SearchResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl SearchResponse {
    pub fn score(&self, id: &str) -> f64 {
        self.hit(id)["_score"].as_f64().unwrap()
    }

    pub fn source(&self, id: &str) -> &Value {
        &self.hit(id)["_source"]
    }

    pub fn max_score(&self) -> f64 {
        self.0["hits"]["max_score"].as_f64().unwrap()
    }

    pub fn max_score_is_null(&self) -> bool {
        self.0["hits"]["max_score"].is_null()
    }

    pub fn total(&self) -> u64 {
        self.0["hits"]["total"]["value"].as_u64().unwrap()
    }

    pub fn total_relation(&self) -> &str {
        self.0["hits"]["total"]["relation"].as_str().unwrap()
    }

    pub fn all_scores_null(&self) -> bool {
        self.hits().iter().all(|h| h["_score"].is_null())
    }

    pub fn all_source_omitted(&self) -> bool {
        self.hits()
            .iter()
            .all(|h| !h.as_object().unwrap().contains_key("_source"))
    }

    pub fn all_sort_omitted(&self) -> bool {
        self.hits()
            .iter()
            .all(|h| !h.as_object().unwrap().contains_key("sort"))
    }

    pub fn sort_values(&self, id: &str) -> &Value {
        &self.hit(id)["sort"]
    }

    pub fn agg(&self, name: &str) -> &Value {
        &self.0["aggregations"][name]
    }

    pub fn agg_value(&self, name: &str) -> f64 {
        self.agg(name)["value"].as_f64().expect("aggregation value")
    }

    pub fn buckets(&self, name: &str) -> &Vec<Value> {
        self.agg(name)["buckets"]
            .as_array()
            .expect("aggregation buckets")
    }

    pub fn hit_ids(&self) -> Vec<String> {
        hit_ids(&self.0)
    }

    fn hits(&self) -> &Vec<Value> {
        self.0["hits"]["hits"].as_array().unwrap()
    }

    fn hit(&self, id: &str) -> &Value {
        self.hits()
            .iter()
            .find(|h| h["_id"] == id)
            .unwrap_or_else(|| panic!("hit {id} not found: {}", self.0))
    }
}

#[derive(Debug)]
pub struct JsonResponse {
    pub status: StatusCode,
    body: Value,
}

impl std::ops::Deref for JsonResponse {
    type Target = Value;

    fn deref(&self) -> &Value {
        &self.body
    }
}

impl std::fmt::Display for JsonResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.body, f)
    }
}

pub async fn to_json(res: Response) -> JsonResponse {
    let status = res.status_code();
    let body = res.json().await.unwrap_or(Value::Null);
    JsonResponse { status, body }
}

async fn into_test_result(res: Response) -> TestResult<Value> {
    let status = res.status_code();
    let text = res.text().await.unwrap_or_default();
    let body = serde_json::from_str(&text).unwrap_or_else(|_| {
        if text.is_empty() {
            Value::Null
        } else {
            json!({ "error": text })
        }
    });

    if status.is_success() {
        Ok(body)
    } else {
        Err(TestErrorResponse { status, body })
    }
}
