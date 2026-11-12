#![allow(dead_code)]

use std::str::FromStr;

use anyhow::Context;
use serde_json::Value as JsonValue;
use sqlx::{
    postgres::{PgArguments, PgConnectOptions, PgPool, PgPoolOptions, PgRow, PgSslMode},
    Arguments, Column as SqlxColumn, Executor, Row as SqlxRow, TypeInfo,
};

use topk_rs::proto::v1::data::{value, Document, Value};

pub struct SqlClient {
    pool: PgPool,
}

impl SqlClient {
    pub(crate) async fn from_env() -> anyhow::Result<Self> {
        let host = std::env::var("PGHOST").unwrap_or_else(|_| {
            let region = std::env::var("TOPK_REGION").expect("TOPK_REGION or PGHOST must be set");
            let host = std::env::var("TOPK_HOST").unwrap_or_else(|_| "topk.io".to_string());
            format!("{region}.sql.{host}")
        });
        let port = std::env::var("PGPORT")
            .unwrap_or("5432".to_string())
            .parse::<u16>()?;
        let user = std::env::var("PGUSER").unwrap_or_else(|_| "default".to_string());
        let password = std::env::var("PGPASSWORD").or_else(|_| std::env::var("TOPK_API_KEY"))?;
        let database = std::env::var("PGDATABASE").unwrap_or_else(|_| "default".to_string());
        let ssl = std::env::var("PGSSLMODE").unwrap_or_else(|_| {
            match std::env::var("TOPK_HTTPS").unwrap_or_else(|_| "true".to_string()) == "true" {
                true => "require".to_string(),
                false => "disable".to_string(),
            }
        });

        let opts = PgConnectOptions::new()
            .host(&host)
            .port(port)
            .username(&user)
            .password(&password)
            .database(&database)
            .ssl_mode(PgSslMode::from_str(&ssl)?);

        let pool = PgPoolOptions::new()
            .max_connections(32)
            .connect_with(opts)
            .await?;

        Ok(Self { pool })
    }

    pub(crate) async fn batch(&self, sqls: &[&str]) -> anyhow::Result<Vec<Document>> {
        let mut conn = self.pool.acquire().await?;
        let mut rows = vec![];
        for sql in sqls {
            rows = conn.fetch_all(sqlx::raw_sql(sql)).await.map_err(|e| {
                match e.as_database_error() {
                    Some(d) => anyhow::anyhow!("{}", d.message()),
                    None => anyhow::anyhow!("{e}"),
                }
            })?;
        }
        Self::map_rows(rows)
    }

    pub(crate) async fn query(&self, sql: &str) -> anyhow::Result<Vec<Document>> {
        let pg_rows: Vec<PgRow> = sqlx::raw_sql(sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| match e.as_database_error() {
                Some(d) => anyhow::anyhow!("{}", d.message()),
                None => anyhow::anyhow!("{e}"),
            })?;

        Self::map_rows(pg_rows)
    }

    pub(crate) async fn prepared(
        &self,
        sql: &str,
        params: Vec<Value>,
    ) -> anyhow::Result<Vec<Document>> {
        let mut args = PgArguments::default();
        for param in params {
            match param.value {
                Some(value::Value::String(v)) => args.add(v).unwrap(),
                Some(value::Value::I32(v)) => args.add(v).unwrap(),
                Some(value::Value::I64(v)) => args.add(v).unwrap(),
                Some(value::Value::F32(v)) => args.add(v).unwrap(),
                Some(value::Value::F64(v)) => args.add(v).unwrap(),
                Some(value::Value::Bool(v)) => args.add(v).unwrap(),
                Some(value::Value::Null(_)) | None => args.add(None::<String>).unwrap(),
                other => anyhow::bail!("unsupported param type: {other:?}"),
            }
        }
        let pg_rows = sqlx::query_with(sql, args)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| match e.as_database_error() {
                Some(d) => anyhow::anyhow!("{}", d.message()),
                None => anyhow::anyhow!("{e}"),
            })?;

        Self::map_rows(pg_rows)
    }

    pub(crate) async fn column_types(&self, sql: &str) -> anyhow::Result<Vec<(String, String)>> {
        let rows: Vec<PgRow> =
            sqlx::raw_sql(sql)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| match e.as_database_error() {
                    Some(d) => anyhow::anyhow!("{}", d.message()),
                    None => anyhow::anyhow!("{e}"),
                })?;
        Ok(match rows.first() {
            Some(row) => row
                .columns()
                .iter()
                .map(|col| {
                    (
                        col.name().to_string(),
                        col.type_info().name().to_lowercase(),
                    )
                })
                .collect(),
            None => vec![],
        })
    }

    pub(crate) async fn prepared_column_types(
        &self,
        sql: &str,
        params: Vec<Value>,
    ) -> anyhow::Result<Vec<(String, String)>> {
        let mut args = PgArguments::default();
        for param in params {
            match param.value {
                Some(value::Value::String(v)) => args.add(v).unwrap(),
                Some(value::Value::I32(v)) => args.add(v).unwrap(),
                Some(value::Value::I64(v)) => args.add(v).unwrap(),
                Some(value::Value::F32(v)) => args.add(v).unwrap(),
                Some(value::Value::F64(v)) => args.add(v).unwrap(),
                Some(value::Value::Bool(v)) => args.add(v).unwrap(),
                Some(value::Value::Null(_)) | None => args.add(None::<String>).unwrap(),
                other => anyhow::bail!("unsupported param type: {other:?}"),
            }
        }
        let rows: Vec<PgRow> = sqlx::query_with(sql, args)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| match e.as_database_error() {
                Some(d) => anyhow::anyhow!("{}", d.message()),
                None => anyhow::anyhow!("{e}"),
            })?;
        Ok(match rows.first() {
            Some(row) => row
                .columns()
                .iter()
                .map(|col| {
                    (
                        col.name().to_string(),
                        col.type_info().name().to_lowercase(),
                    )
                })
                .collect(),
            None => vec![],
        })
    }

    fn map_rows(pg_rows: Vec<PgRow>) -> anyhow::Result<Vec<Document>> {
        pg_rows
            .iter()
            .map(|row| {
                row.columns()
                    .iter()
                    .map(|col| {
                        let ordinal = col.ordinal();
                        let type_name = col.type_info().name().to_lowercase();
                        let value = (|| -> anyhow::Result<Value> {
                            Ok(match type_name.as_str() {
                                "json" | "jsonb" => {
                                    match row.try_get::<Option<JsonValue>, _>(ordinal)? {
                                        None | Some(JsonValue::Null) => Value::null(),
                                        Some(JsonValue::String(s)) => Value::string(s),
                                        Some(JsonValue::Bool(b)) => Value::bool(b),
                                        Some(JsonValue::Number(n)) => match n.as_i64() {
                                            Some(i) => Value::i64(i),
                                            None => Value::f64(n.as_f64().context(
                                                "JSON number is not representable as f64",
                                            )?),
                                        },
                                        Some(other) => Value::string(other.to_string()),
                                    }
                                }
                                "int2" => row
                                    .try_get::<Option<i16>, _>(ordinal)?
                                    .map(|v| Value::i64(v.into()))
                                    .unwrap_or_else(Value::null),
                                "int4" => row
                                    .try_get::<Option<i32>, _>(ordinal)?
                                    .map(|v| Value::i64(v.into()))
                                    .unwrap_or_else(Value::null),
                                "int8" => row
                                    .try_get::<Option<i64>, _>(ordinal)?
                                    .map(Value::i64)
                                    .unwrap_or_else(Value::null),
                                "float4" => row
                                    .try_get::<Option<f32>, _>(ordinal)?
                                    .map(Value::f32)
                                    .unwrap_or_else(Value::null),
                                "float8" => row
                                    .try_get::<Option<f64>, _>(ordinal)?
                                    .map(Value::f64)
                                    .unwrap_or_else(Value::null),
                                "bool" => row
                                    .try_get::<Option<bool>, _>(ordinal)?
                                    .map(Value::bool)
                                    .unwrap_or_else(Value::null),
                                "bytea" => row
                                    .try_get::<Option<Vec<u8>>, _>(ordinal)?
                                    .map(Value::bytes)
                                    .unwrap_or_else(Value::null),
                                _ => row
                                    .try_get::<Option<String>, _>(ordinal)?
                                    .map(Value::string)
                                    .unwrap_or_else(Value::null),
                            })
                        })()
                        .with_context(|| {
                            format!("decoding column {:?} ({type_name})", col.name())
                        })?;
                        Ok((col.name().to_string(), value))
                    })
                    .collect::<anyhow::Result<Vec<_>>>()
                    .map(Document::from)
            })
            .collect()
    }
}
