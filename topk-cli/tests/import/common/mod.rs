mod output;

pub mod seed;

use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use clap::{Args, Command, FromArgMatches};
use futures::TryStreamExt;
use serde_json::Value;
use tempfile::TempDir;
use test_context::AsyncTestContext;

pub use crate::common::command::TestCommand;
use crate::common::session::CliTestContext;
use topk::auth::OAuthConfig;
use topk::client::DataClient;
use topk::config::Config;
use topk::endpoint::DataEndpoint;
use topk::host::Host;
use topk::import::source::{Source, Uri};
use topk::import::spec::{Field, Spec, Target};
use topk::import::state::{DestinationId, State};
use topk::import::Error;
use topk_rs::doc;
use topk_rs::proto::v1::control::FieldSpec;
use topk_rs::proto::v1::data::{ConsistencyLevel, Document};

/// A flag group as the CLI reads it from the environment.
pub fn from_env<T: Args + FromArgMatches>() -> T {
    let matches = T::augment_args(Command::new("test")).get_matches_from(["test"]);
    T::from_arg_matches(&matches).expect("arguments from the environment")
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

pub async fn source(locator: &str) -> Result<Source, Error> {
    locator.parse::<Uri>()?.connect().await
}

pub async fn json_documents(
    source: &Source,
    target: &Target,
) -> Result<Vec<(Option<String>, Value)>, Error> {
    source
        .documents(target)
        .await?
        .map_ok(|(partition, doc)| (partition, Value::Object(json(&doc))))
        .try_collect()
        .await
}

pub fn target<'a>(
    from: &str,
    id: &str,
    fields: impl IntoIterator<Item = (&'a str, FieldSpec)>,
) -> Target {
    Target {
        from: from.into(),
        id: id.into(),
        fields: fields
            .into_iter()
            .map(|(name, schema)| (name.into(), Field::new(schema)))
            .collect(),
        filter: None,
        partition: None,
        limit: None,
    }
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
    config_dir: PathBuf,
}

impl Default for Scratch {
    fn default() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("it")
            .rand_bytes(16)
            .tempdir()
            .unwrap();
        Self {
            config_dir: dir.path().join("config"),
            dir,
        }
    }
}

impl Scratch {
    pub fn scratch(&self) -> &Path {
        self.dir.path()
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    pub fn name(&self, name: &str) -> String {
        format!(
            "{}_{name}",
            self.dir
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_ascii_lowercase()
        )
    }

    pub fn command(&self) -> TestCommand {
        TestCommand::new().env_remove("TOPK_API_KEY").envs([
            ("TOPK_CONFIG_DIR", self.config_dir.clone()),
            ("TOPK_IMPORT_STATE_DIR", self.dir.path().join("state")),
        ])
    }

    pub fn state_path(&self, id: &str) -> PathBuf {
        self.path(&format!("state/{id}.toml"))
    }

    pub fn file(&self, name: &str, content: &str) -> String {
        let path = self.path(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, content).unwrap();
        path.to_str().unwrap().to_string()
    }

    pub fn read(&self, name: &str) -> String {
        std::fs::read_to_string(self.path(name)).unwrap()
    }

    pub fn jsonl(&self, name: &str, docs: &[Document]) -> String {
        let lines = docs
            .iter()
            .map(|doc| Value::Object(json(doc)).to_string())
            .collect::<Vec<_>>();
        self.file(name, &lines.join("\n"))
    }

    pub fn parquet(&self, name: &str, select: &str) -> String {
        let path = self.path(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        duckdb::Connection::open_in_memory()
            .unwrap()
            .execute_batch(&format!(
                "COPY ({select}) TO '{}' (FORMAT parquet);",
                path.display().to_string().replace('\'', "''")
            ))
            .unwrap();
        path.display().to_string()
    }

    pub fn seed_parquet(&self, name: &str, docs: Vec<Document>) -> String {
        let input = self.jsonl(&format!("{name}.jsonl"), &docs);
        self.parquet(
            &format!("{name}.parquet"),
            &format!(
                "SELECT * FROM read_json_auto('{}')",
                input.replace('\'', "''")
            ),
        )
    }
}

impl AsyncTestContext for Scratch {
    async fn setup() -> Self {
        Self::default()
    }
}

pub struct Ctx {
    pub cli: Scratch,
    _session: CliTestContext,
    pub client: DataClient,
    used: RefCell<HashSet<String>>,
}

impl Ctx {
    pub fn state(&self, source: &Source, spec: Spec) -> State {
        State::new(
            source.locator().into(),
            DestinationId {
                project_id: std::env::var("TOPK_PROJECT_ID").unwrap().parse().unwrap(),
                region: self.client.config().region().unwrap().into(),
            },
            spec,
        )
    }

    pub fn collection(&self, name: &str) -> String {
        let full = self.cli.name(name);
        self.used.borrow_mut().insert(full.clone());
        full
    }

    /// The documents with these ids, as JSON in id order.
    pub async fn docs(
        &self,
        collection: &str,
        ids: impl IntoIterator<Item = impl ToString>,
    ) -> Vec<Value> {
        let docs: BTreeMap<String, Value> = self
            .client
            .collection(collection)
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
        let mut cli = Scratch::default();
        cli.config_dir = Config::dir().expect("session config directory");
        Ctx {
            cli,
            _session: session,
            client: DataClient::new(
                &config,
                project_id.clone(),
                from_env::<DataEndpoint>().region().expect("TOPK_REGION"),
            )
            .expect("data client"),
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
