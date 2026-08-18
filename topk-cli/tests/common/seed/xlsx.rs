use tempfile::TempDir;
use topk::import::Target;
use topk_rs::proto::v1::data::Document;

use crate::common::jsonl;

pub struct File {
    dir: TempDir,
    conn: duckdb::Connection,
}

impl File {
    pub fn new() -> anyhow::Result<File> {
        let conn = duckdb::Connection::open_in_memory()?;
        conn.execute_batch("INSTALL excel; LOAD excel;")?;
        Ok(File {
            dir: tempfile::tempdir()?,
            conn,
        })
    }
}

#[async_trait::async_trait(?Send)]
impl super::Seed for File {
    async fn seed(&self, name: &str, docs: Vec<Document>) -> anyhow::Result<Target> {
        let file = jsonl(&docs);
        let path = self.dir.path().join(format!("{name}.xlsx"));
        self.conn.execute_batch(&format!(
            "COPY (SELECT * FROM read_json_auto('{}')) TO '{}' (FORMAT xlsx, HEADER true);",
            file.path().display(),
            path.display()
        ))?;
        Ok(super::discovered(
            Target {
                from: path.display().to_string(),
                ..Default::default()
            },
            None,
        )
        .await?)
    }
}
