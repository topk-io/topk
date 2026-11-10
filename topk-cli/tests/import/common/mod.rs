#[macro_use]
mod output;
pub use output::CliOutput;

pub mod gate;
pub mod seed;

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Display;
use std::io::Write as _;
use std::path::Path;

use clap::{Args, Command, FromArgMatches};
use futures::StreamExt;
use indexmap::IndexMap;
use indicatif::{MultiProgress, ProgressDrawTarget};
use serde_json::Value;
use tempfile::{NamedTempFile, TempDir};
use test_context::AsyncTestContext;
use uuid::Uuid;

pub use crate::common::command::TestCommand;
use crate::common::session::CliTestContext;
use topk::auth::OAuthConfig;
use topk::client::DataClient;
use topk::config::Config;
use topk::endpoint::DataEndpoint;
use topk::host::Host;
use topk::import::{Error, Spec, State, Target, Uri};
use topk_rs::doc;
use topk_rs::proto::v1::control::FieldSpec;
use topk_rs::proto::v1::data::{ConsistencyLevel, Document};
use topk_rs::{Client, CollectionClient};

/// A flag group as the CLI reads it from the environment.
pub fn from_env<T: Args + FromArgMatches>() -> T {
    let matches = T::augment_args(Command::new("test")).get_matches_from(["test"]);
    T::from_arg_matches(&matches).expect("arguments from the environment")
}

/// A flag group's defaults, ignoring the environment.
pub fn defaults<T: Args + FromArgMatches>() -> T {
    let matches = T::augment_args(Command::new("test"))
        .mut_args(|arg| arg.env(None::<&str>))
        .get_matches_from(["test"]);
    T::from_arg_matches(&matches).expect("default arguments")
}

/// The production API and issuer; file sources never reach either.
pub fn config() -> Config {
    Config::new(
        defaults::<Host>(),
        defaults::<OAuthConfig>(),
        std::env::temp_dir(),
    )
}

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

/// Where `topk import` keeps run state for this test process — never the
/// user's real config dir, which failing-on-purpose runs would litter.
pub fn state_dir() -> &'static Path {
    static DIR: std::sync::OnceLock<TempDir> = std::sync::OnceLock::new();
    DIR.get_or_init(|| tempfile::tempdir().unwrap()).path()
}

pub fn topk() -> TestCommand {
    TestCommand::new()
        .env_remove("TOPK_API_KEY")
        .envs([("TOPK_IMPORT_STATE_DIR", state_dir())])
}

/// The documents the library reads for `target`.
pub async fn stream(
    url: Option<&str>,
    target: &Target,
) -> Result<Vec<(Option<String>, Value)>, Error> {
    let uri: Uri = match url {
        Some(url) => url.parse()?,
        None => target.from.parse()?,
    };
    let source = topk::import::Source::connect(&uri, &config()).await?;
    let mut rows = Box::pin(topk::import::documents(&source, target)?);
    let mut out = Vec::new();
    while let Some(row) = rows.next().await {
        let (partition, doc) = row?;
        let map = json(&doc);
        out.push((partition, Value::Object(map)));
    }
    Ok(out)
}

/// A one-collection spec as TOML; `fields` is the body of `[c.fields]`.
pub fn spec_toml(from: &str, id: &str, fields: &str) -> String {
    format!("[c]\nfrom = {from:?}\nid = {id:?}\n\n[c.fields]\n{fields}\n")
}

pub fn spec(collection: &str, target: Target) -> Spec {
    Spec {
        collections: IndexMap::from([(collection.to_string(), target)]),
    }
}

/// A target through the real spec parser, so a test cannot build one the TOML
/// layer would reject.
#[track_caller]
pub fn target(from: &str, id: &str, fields: &str) -> Target {
    let mut spec = toml::from_str::<Spec>(&spec_toml(from, id, fields)).expect("spec parses");
    spec.collections.shift_remove("c").expect("target c")
}

#[track_caller]
pub fn refused<T, E: std::error::Error + Send + Sync + 'static>(result: Result<T, E>) -> String {
    match result {
        Err(e) => format!("{:#}", anyhow::Error::new(e)),
        Ok(_) => panic!("expected a refusal"),
    }
}

pub struct Scratch {
    dir: TempDir,
}

impl Scratch {
    pub fn scratch(&self) -> &Path {
        self.dir.path()
    }

    pub fn topk(&self) -> TestCommand {
        topk()
    }

    pub fn state_path(&self, id: impl Display) -> std::path::PathBuf {
        state_dir().join(format!("{id}.toml"))
    }

    pub fn state(&self, id: impl Display) -> State {
        toml::from_str(&std::fs::read_to_string(self.state_path(id)).expect("run state"))
            .expect("run state parses")
    }

    pub fn save(&self, state: &State) {
        std::fs::write(
            self.state_path(&state.id),
            toml::to_string_pretty(state).expect("state serializes"),
        )
        .unwrap();
    }

    pub fn spec_file(&self, toml: &str) -> String {
        let path = self
            .dir
            .path()
            .join(format!("{}.toml", unique_name("spec")));
        std::fs::write(&path, toml).unwrap();
        path.to_str().unwrap().to_string()
    }

    pub fn multi_spec(&self, targets: impl IntoIterator<Item = (String, Target)>) -> String {
        let spec = Spec {
            collections: targets.into_iter().collect(),
        };
        self.spec_file(&toml::to_string_pretty(&spec).expect("spec serializes"))
    }

    pub fn sql_parquet(&self, stem: &str, select: &str) -> String {
        let path = self.dir.path().join(format!("{stem}.parquet"));
        duckdb::Connection::open_in_memory()
            .unwrap()
            .execute_batch(&format!(
                "COPY ({select}) TO '{}' (FORMAT parquet);",
                path.display()
            ))
            .unwrap();
        path.display().to_string()
    }

    pub fn parquet(&self, select: &str) -> String {
        self.sql_parquet(&unique_name("data"), select)
    }

    pub fn csv(&self, name: &str, text: &str) -> String {
        let path = self.dir.path().join(format!("{name}.csv"));
        std::fs::write(
            &path,
            text.lines().map(str::trim).collect::<Vec<_>>().join("\n") + "\n",
        )
        .unwrap();
        path.display().to_string()
    }

    pub async fn seed_parquet(&self, name: &str, docs: Vec<Document>) -> String {
        seed::parquet::write(self.dir.path(), name, &docs)
            .await
            .unwrap()
    }

    pub fn spec(&self, collection: &str, target: Target) -> String {
        self.multi_spec([(collection.to_string(), target)])
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
    pub config: Config,
    pub progress: MultiProgress,
    scratch: Scratch,
    _session: CliTestContext,
    client: DataClient,
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

    /// The documents with these ids, as JSON in id order.
    pub async fn docs(
        &self,
        collection: &str,
        ids: impl IntoIterator<Item = impl ToString>,
    ) -> Vec<Value> {
        Self::get(self.client.collection(collection), ids).await
    }

    pub async fn docs_in(
        &self,
        partition: &str,
        collection: &str,
        ids: impl IntoIterator<Item = impl ToString>,
    ) -> Vec<Value> {
        Self::get(self.client.collection(collection).partition(partition), ids).await
    }

    async fn get(
        collection: CollectionClient,
        ids: impl IntoIterator<Item = impl ToString>,
    ) -> Vec<Value> {
        let docs: BTreeMap<String, Value> = collection
            .get(
                ids.into_iter().map(|id| id.to_string()),
                None,
                None,
                Some(ConsistencyLevel::Strong),
            )
            .await
            .expect("get documents")
            .into_iter()
            .map(|(id, doc)| {
                let doc = doc
                    .into_iter()
                    .map(|(key, value)| (key, value.try_into().expect("json value")))
                    .collect();
                (id, Value::Object(doc))
            })
            .collect();
        docs.into_values().collect()
    }

    pub async fn exists(&self, collection: &str) -> bool {
        match self.client.collections().get(collection).await {
            Ok(_) => true,
            Err(topk_rs::Error::CollectionNotFound) => false,
            Err(e) => panic!("get collection {collection:?}: {e}"),
        }
    }

    pub async fn schema(&self, collection: &str) -> HashMap<String, FieldSpec> {
        self.client
            .collections()
            .get(collection)
            .await
            .expect("get collection")
            .schema
    }
}

impl AsyncTestContext for Ctx {
    async fn setup() -> Self {
        let session = CliTestContext::logged_out();
        session.login().await;
        let project_id = session.select_e2e_project();
        std::env::set_var("TOPK_PROJECT_ID", project_id.as_str());
        let config = Config::new(
            from_env::<Host>(),
            from_env::<OAuthConfig>(),
            Config::dir().expect("config directory"),
        );
        Ctx {
            config: config.clone(),
            progress: MultiProgress::with_draw_target(ProgressDrawTarget::hidden()),
            scratch: Scratch::setup().await,
            _session: session,
            client: DataClient::new(
                &config,
                project_id.clone(),
                from_env::<DataEndpoint>().region().expect("TOPK_REGION"),
            )
            .expect("data client"),
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

pub async fn catalog_of(locator: &str) -> Result<Vec<topk::import::Table>, Error> {
    let uri: Uri = locator.parse()?;
    topk::import::Source::connect(&uri, &config())
        .await?
        .catalog()
        .await
}

pub async fn discover_err(locator: &str, pattern: &str) -> String {
    let result = match catalog_of(locator).await {
        Ok(catalog) => topk::import::discover(&catalog, &[pattern.to_string()], None, None),
        Err(error) => Err(error),
    };
    match result {
        Err(error) => format!("{:#}", anyhow::Error::new(error)),
        Ok(discovered) => panic!(
            "expected discover to fail, got {} collection(s)",
            discovered.collections.len()
        ),
    }
}
