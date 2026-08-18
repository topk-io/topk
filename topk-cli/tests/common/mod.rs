#![allow(dead_code)]

pub mod seed;

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Write as _;
use std::path::Path;
use std::process::Output;

use futures::StreamExt;
use indexmap::IndexMap;
use tempfile::{NamedTempFile, TempDir};
use test_context::AsyncTestContext;
use topk::import::{Field, Spec, Target};
use topk_rs::doc;
use topk_rs::proto::v1::data::{ConsistencyLevel, Document, Value};
use topk_rs::{Client, ClientConfig};
use uuid::Uuid;

pub fn books() -> Vec<Document> {
    vec![
        doc!(
            "_id" => "mockingbird",
            "title" => "To Kill a Mockingbird",
            "author" => "Lee",
            "published_year" => 1960_u32,
            "rating" => 4.3,
            "in_print" => true
        ),
        doc!(
            "_id" => "nineteen_eighty_four",
            "title" => "1984",
            "author" => "Orwell",
            "published_year" => 1949_u32,
            "rating" => 4.2,
            "in_print" => true
        ),
        doc!(
            "_id" => "pride",
            "title" => "Pride and Prejudice",
            "author" => "Austen",
            "published_year" => 1813_u32,
            "rating" => 4.3,
            "in_print" => false
        ),
    ]
}

pub fn rows(start: u64, count: u64) -> Vec<Document> {
    (start..start + count)
        .map(|i| doc!("_id" => i.to_string(), "name" => format!("r{i}")))
        .collect()
}

pub fn unique_name(prefix: &str) -> String {
    format!("{prefix}_{}", &Uuid::new_v4().simple().to_string()[..8])
}

pub fn json(doc: &Document) -> serde_json::Map<String, serde_json::Value> {
    doc.fields
        .iter()
        .map(|(key, value)| {
            (
                key.clone(),
                serde_json::Value::try_from(value.clone()).expect("json value"),
            )
        })
        .collect()
}

pub fn jsonl(docs: &[Document]) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    for doc in docs {
        writeln!(file, "{}", serde_json::Value::Object(json(doc))).unwrap();
    }
    file.flush().unwrap();
    file
}

fn run(args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_topk"));
    cmd.args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.output().expect("run topk")
}

pub fn ok(args: &[&str], env: &[(&str, &str)]) -> String {
    let out = run(args, env);
    assert!(
        out.status.success(),
        "`topk {}` failed:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

pub fn fails(args: &[&str], env: &[(&str, &str)]) -> String {
    let out = run(args, env);
    assert!(
        !out.status.success(),
        "`topk {}` should have failed",
        args.join(" ")
    );
    String::from_utf8_lossy(&out.stderr).into_owned()
}

pub async fn discover_spec(locator: &str, pattern: Option<&str>) -> Spec {
    let uri = locator.parse().expect("source uri parses");
    let patterns: Vec<String> = pattern.into_iter().map(str::to_string).collect();
    topk::import::discover(&uri, &patterns, None, None)
        .await
        .expect("discover")
}

pub async fn stream_docs(
    target: &Target,
) -> Result<BTreeMap<String, serde_json::Value>, topk::import::Error> {
    stream_docs_from(None, target).await
}

pub async fn stream_docs_from(
    url: Option<&str>,
    target: &Target,
) -> Result<BTreeMap<String, serde_json::Value>, topk::import::Error> {
    let uri = match url {
        Some(url) => url.parse()?,
        None => target.from.parse()?,
    };
    let source = topk::import::connect(&uri).await?;
    let mut rows = Box::pin(topk::import::documents(&source, target).await?);
    let mut out = BTreeMap::new();
    while let Some(doc) = rows.next().await {
        let map = json(&doc?);
        let id = map["_id"].as_str().expect("string id").to_string();
        out.insert(id, serde_json::Value::Object(map));
    }
    Ok(out)
}

pub fn fields<const N: usize>(entries: [(&str, Field); N]) -> IndexMap<String, Field> {
    entries
        .into_iter()
        .map(|(name, field)| (name.to_string(), field))
        .collect()
}

pub fn discover(source: &str, stream: Option<&str>) -> String {
    let mut args = vec!["import", source, "--dry-run"];
    if let Some(s) = stream {
        args.push(s);
    }
    ok(&args, &[])
}

/// `topk import [<source>] -f <spec> …`; the source is required for anything
/// that is not a file, since a spec never carries one.
pub fn import_args<'a>(url: Option<&'a str>, spec: &'a str, extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec!["import"];
    args.extend(url);
    args.extend(["-f", spec]);
    args.extend(extra.iter().copied());
    args
}

pub fn dry_run(spec: &str, extra: &[&str]) -> BTreeMap<String, serde_json::Value> {
    dry_run_from(None, spec, extra)
}

/// Dry-run documents land on stderr; the spec is the stdout artifact.
pub fn dry_run_from(
    url: Option<&str>,
    spec: &str,
    extra: &[&str],
) -> BTreeMap<String, serde_json::Value> {
    let mut extra = extra.to_vec();
    extra.push("--dry-run");
    let args = import_args(url, spec, &extra);
    let out = run(&args, &[]);
    assert!(out.status.success(), "`topk {}` failed", args.join(" "));
    String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter(|l| l.trim_start().starts_with('{'))
        .map(|l| {
            let doc: serde_json::Value = serde_json::from_str(l).expect("dry-run json");
            (doc["_id"].as_str().unwrap().to_string(), doc)
        })
        .collect()
}

pub struct Scratch {
    dir: TempDir,
}

impl Scratch {
    pub fn scratch(&self) -> &Path {
        self.dir.path()
    }

    pub fn spec_file(&self, toml: &str) -> String {
        let path = self.dir.path().join("spec.toml");
        std::fs::write(&path, toml).unwrap();
        path.to_str().unwrap().to_string()
    }

    pub fn target_spec(&self, name: &str, target: Target) -> String {
        self.multi_spec([(name.to_string(), target)])
    }

    pub fn multi_spec(&self, targets: impl IntoIterator<Item = (String, Target)>) -> String {
        let spec = Spec {
            collections: targets.into_iter().collect(),
        };
        self.spec_file(&toml::to_string_pretty(&spec).expect("spec serializes"))
    }

    pub fn sql_parquet(&self, stem: &str, select: &str) -> String {
        let path = self.dir.path().join(format!("{stem}.parquet"));
        let conn = duckdb::Connection::open_in_memory().unwrap();
        conn.execute_batch(&format!(
            "COPY ({select}) TO '{}' (FORMAT parquet);",
            path.display()
        ))
        .unwrap();
        path.display().to_string()
    }

    pub async fn seed_parquet(&self, name: &str, docs: Vec<Document>) -> Target {
        seed::parquet::write(self.dir.path(), name, &docs)
            .await
            .unwrap()
    }
}

impl AsyncTestContext for Scratch {
    async fn setup() -> Self {
        Scratch {
            dir: tempfile::tempdir().unwrap(),
        }
    }
}

pub struct Ctx {
    scratch: Scratch,
    client: Client,
    scope: String,
    used: RefCell<HashSet<String>>,
}

impl std::ops::Deref for Ctx {
    type Target = Scratch;

    fn deref(&self) -> &Scratch {
        &self.scratch
    }
}

impl Ctx {
    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn collection(&self, name: &str) -> String {
        let full = format!("{}-{name}", self.scope);
        self.used.borrow_mut().insert(full.clone());
        full
    }

    pub async fn get(
        &self,
        collection: &str,
        ids: &[&str],
    ) -> HashMap<String, HashMap<String, Value>> {
        self.client
            .collection(collection)
            .get(
                ids.iter().copied(),
                None,
                None,
                Some(ConsistencyLevel::Strong),
            )
            .await
            .expect("get documents")
    }
}

impl AsyncTestContext for Ctx {
    async fn setup() -> Self {
        let region = std::env::var("TOPK_REGION").expect("TOPK_REGION not set");
        let api_key = std::env::var("TOPK_API_KEY").expect("TOPK_API_KEY not set");
        let host = std::env::var("TOPK_HOST").unwrap_or_else(|_| "topk.io".to_string());
        let https = std::env::var("TOPK_HTTPS")
            .map(|v| v == "true")
            .unwrap_or(true);
        Ctx {
            scratch: Scratch::setup().await,
            client: Client::new(
                ClientConfig::new(api_key, region)
                    .with_host(host)
                    .with_https(https),
            ),
            scope: format!("it-{}", Uuid::new_v4()),
            used: RefCell::new(HashSet::new()),
        }
    }

    async fn teardown(self) {
        for name in self.used.take() {
            match self.client.collections().delete(&name).await {
                Ok(()) | Err(topk_rs::Error::CollectionNotFound) => {}
                Err(e) => println!("Teardown error deleting collection {name:?}: {e}"),
            }
        }
    }
}

pub fn field(doc: &HashMap<String, Value>, name: &str) -> serde_json::Value {
    serde_json::Value::try_from(doc.get(name).expect("field present").clone()).unwrap()
}

pub fn doc_json(doc: &HashMap<String, Value>) -> serde_json::Value {
    serde_json::Value::Object(
        doc.iter()
                .map(|(key, value)| {
                (
                    key.clone(),
                    serde_json::Value::try_from(value.clone()).expect("json value"),
                )
            })
            .collect(),
    )
}
