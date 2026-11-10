pub const ENDPOINT: &str = "http://localhost:9100";

pub fn seed(name: &str, input: &str) -> anyhow::Result<String> {
    let conn = duckdb::Connection::open_in_memory()?;
    conn.execute_batch(
        "INSTALL httpfs; LOAD httpfs; \
             CREATE SECRET (TYPE s3, KEY_ID 'rustfsadmin', SECRET 'rustfsadmin', \
             ENDPOINT 'localhost:9100', URL_STYLE 'path', USE_SSL false);",
    )?;
    std::process::Command::new("curl")
        .args([
            "-s",
            "-X",
            "PUT",
            &format!("{}/topk-it", ENDPOINT),
            "--aws-sigv4",
            "aws:amz:us-east-1:s3",
            "--user",
            "rustfsadmin:rustfsadmin",
        ])
        .output()?;

    conn.execute_batch(&format!(
            "COPY (SELECT * FROM read_json_auto('{}')) TO 's3://topk-it/{name}.parquet' (FORMAT parquet);",
            input.replace('\'', "''")
        ))?;

    Ok(format!("s3://topk-it/{name}.parquet"))
}
