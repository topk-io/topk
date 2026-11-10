use std::path::Path;

use tempfile::TempDir;
use topk_rs::proto::v1::data::Document;

use crate::import::common::jsonl;

pub struct File {
    dir: TempDir,
}

impl File {
    pub fn new() -> anyhow::Result<File> {
        Ok(File {
            dir: tempfile::tempdir()?,
        })
    }
}

pub async fn write(dir: &Path, name: &str, docs: &[Document]) -> anyhow::Result<String> {
    let file = jsonl(docs);
    let path = dir.join(format!("{name}.parquet"));
    let conn = duckdb::Connection::open_in_memory()?;
    conn.execute_batch(&format!(
        "COPY (SELECT * FROM read_json_auto('{}')) TO '{}' (FORMAT parquet);",
        file.path().display(),
        path.display()
    ))?;
    Ok(path.display().to_string())
}

#[async_trait::async_trait(?Send)]
impl super::Seed for File {
    async fn seed(&self, name: &str, docs: Vec<Document>) -> anyhow::Result<String> {
        write(self.dir.path(), name, &docs).await
    }
}
