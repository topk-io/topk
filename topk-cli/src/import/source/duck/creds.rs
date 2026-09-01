use std::sync::OnceLock;

use duckdb::Connection;

use crate::import::error::Error;

use super::file::ObjectStore;
use super::{extension_error, lit, strip_sql};

/// A synthetic AWS profile whose `credential_process` is `aws configure
/// export-credentials`: the aws CLI resolves SSO → assume-role, which duckdb's
/// C++ chain cannot, and `REFRESH auto` re-runs it on expiry. None when env
/// keys are set, no `aws` is on PATH, or there is no profile. Sets
/// `AWS_CONFIG_FILE`, so `main` runs it before the runtime spawns threads.
pub fn aws_process_profile() -> Option<&'static str> {
    static PROFILE: OnceLock<Option<String>> = OnceLock::new();
    PROFILE
        .get_or_init(|| {
            if std::env::var("AWS_ACCESS_KEY_ID").is_ok() {
                return None;
            }
            let aws = std::env::split_paths(&std::env::var_os("PATH")?)
                .map(|dir| dir.join("aws"))
                .find(|p| p.is_file())?;
            let profile = std::env::var("AWS_PROFILE").unwrap_or_else(|_| "default".into());
            // The inner command must read the real config, not the synthetic one.
            let inner = match std::env::var("AWS_CONFIG_FILE") {
                Ok(original) => format!("AWS_CONFIG_FILE='{original}'"),
                Err(_) => {
                    // No profile anywhere: leave the default chain alone.
                    if !dirs::home_dir()?.join(".aws/config").is_file() {
                        return None;
                    }
                    "-u AWS_CONFIG_FILE".to_string()
                }
            };
            let path = std::env::temp_dir().join(format!("topk-aws-{}.ini", std::process::id()));
            std::fs::write(
                &path,
                format!(
                    "[profile topk-import]\ncredential_process = env {inner} '{}' \
                     configure export-credentials --profile '{profile}' --format process\n",
                    aws.display()
                ),
            )
            .ok()?;
            std::env::set_var("AWS_CONFIG_FILE", &path);
            Some("topk-import".to_string())
        })
        .as_deref()
}

pub(super) fn secret(conn: &Connection, store: &ObjectStore) -> Result<(), Error> {
    // `REFRESH auto`: everything but a static key pair expires within the run.
    match store {
        ObjectStore::S3 => match std::env::var("AWS_ENDPOINT_URL") {
            Ok(endpoint) => {
                let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".into());
                let use_ssl = endpoint.starts_with("https");
                let host = lit(endpoint
                    .trim_start_matches("https://")
                    .trim_start_matches("http://"));
                let region = lit(&region);
                conn.execute_batch(&format!(
                    "CREATE OR REPLACE SECRET s3 (TYPE s3, PROVIDER credential_chain, \
                     REFRESH auto, CHAIN 'env', REGION '{region}', ENDPOINT '{host}', \
                     URL_STYLE 'path', USE_SSL {use_ssl});"
                ))?;
            }
            Err(_) => match aws_process_profile() {
                Some(profile) => conn
                    .execute_batch(&format!(
                        "CREATE OR REPLACE SECRET s3 (TYPE s3, PROVIDER credential_chain, \
                         CHAIN 'process', PROFILE '{profile}', REFRESH auto);"
                    ))
                    .map_err(|e| {
                        Error::InvalidArgument(format!(
                            "{} — if your AWS profile uses SSO, run `aws sso login`",
                            strip_sql(&e)
                        ))
                    })?,
                None => conn.execute_batch(
                    "CREATE OR REPLACE SECRET s3 (TYPE s3, PROVIDER credential_chain, REFRESH auto);",
                )?,
            },
        },
        ObjectStore::Gcs => {
            conn.execute_batch(
                "CREATE OR REPLACE SECRET gcs (TYPE gcs, PROVIDER credential_chain, REFRESH auto);",
            )?;
        }
        ObjectStore::HuggingFace => {
            conn.execute_batch(
                "CREATE OR REPLACE SECRET hf (TYPE huggingface, PROVIDER credential_chain);",
            )?;
        }
        ObjectStore::Azure => {
            conn.execute_batch("INSTALL azure; LOAD azure;")
                .map_err(|e| extension_error("azure", e))?;
            // The azure extension rejects REFRESH.
            conn.execute_batch(
                "CREATE OR REPLACE SECRET az (TYPE azure, PROVIDER credential_chain);",
            )?;
        }
        // httpfs alone; anonymous GET.
        ObjectStore::Http => {}
    }
    Ok(())
}
