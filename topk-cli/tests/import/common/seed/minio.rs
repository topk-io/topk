use topk::import::Target;
use topk_rs::proto::v1::data::Document;

use crate::common::jsonl;

pub struct S3 {
    conn: duckdb::Connection,
}

impl S3 {
    pub const ENDPOINT: &str = "http://localhost:9100";

    pub const ENV: &[(&str, &str)] = &[
        ("AWS_ENDPOINT_URL", Self::ENDPOINT),
        ("AWS_ACCESS_KEY_ID", "minioadmin"),
        ("AWS_SECRET_ACCESS_KEY", "minioadmin"),
        ("AWS_REGION", "us-east-1"),
    ];

    pub fn new() -> anyhow::Result<S3> {
        let conn = duckdb::Connection::open_in_memory()?;
        conn.execute_batch(
            "INSTALL httpfs; LOAD httpfs; \
             CREATE SECRET (TYPE s3, KEY_ID 'minioadmin', SECRET 'minioadmin', \
             ENDPOINT 'localhost:9100', URL_STYLE 'path', USE_SSL false);",
        )?;
        Ok(S3 { conn })
    }
}

#[async_trait::async_trait(?Send)]
impl super::Seed for S3 {
    async fn seed(&self, name: &str, docs: Vec<Document>) -> anyhow::Result<Target> {
        std::process::Command::new("curl")
            .args([
                "-s",
                "-X",
                "PUT",
                &format!("{}/topk-it", Self::ENDPOINT),
                "--aws-sigv4",
                "aws:amz:us-east-1:s3",
                "--user",
                "minioadmin:minioadmin",
            ])
            .output()?;

        let file = jsonl(&docs);
        self.conn.execute_batch(&format!(
            "COPY (SELECT * FROM read_json_auto('{}')) TO 's3://topk-it/{name}.parquet' (FORMAT parquet);",
            file.path().display()
        ))?;

        // In-process discovery reads the object through duckdb's httpfs, which
        // takes its credentials from the environment.
        for (key, value) in Self::ENV {
            std::env::set_var(key, value);
        }
        Ok(super::discovered(
            Target {
                from: format!("s3://topk-it/{name}.parquet"),
                ..Default::default()
            },
            None,
        )
        .await?)
    }
}
