#![allow(dead_code)]

use std::str::FromStr;

use serde_json::Value as JsonValue;
use sqlx::{
    Arguments, Column as SqlxColumn, Executor, Row as SqlxRow, TypeInfo,
    postgres::{PgArguments, PgConnectOptions, PgPool, PgPoolOptions, PgRow, PgSslMode},
};

use topk_rs::proto::v1::data::{Document, Value, value};

pub(crate) struct SqlClient {
    pool: PgPool,
}

impl SqlClient {
    pub(crate) async fn from_env() -> anyhow::Result<Self> {
        let host = std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("POSTGRES_PORT")
            .unwrap_or("5432".to_string())
            .parse::<u16>()?;
        let user = std::env::var("POSTGRES_USER").unwrap_or_else(|_| "default".to_string());
        let password =
            std::env::var("POSTGRES_PASSWORD").or_else(|_| std::env::var("TOPK_API_KEY"))?;
        let ssl = std::env::var("POSTGRES_SSL").unwrap_or_else(|_| "prefer".to_string());

        let opts = PgConnectOptions::new()
            .host(&host)
            .port(port)
            .username(&user)
            .password(&password)
            .database("default")
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
        Ok(Self::map_rows(rows))
    }

    pub(crate) async fn query(&self, sql: &str) -> anyhow::Result<Vec<Document>> {
        let pg_rows: Vec<PgRow> = sqlx::raw_sql(sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| match e.as_database_error() {
                Some(d) => anyhow::anyhow!("{}", d.message()),
                None => anyhow::anyhow!("{e}"),
            })?;

        Ok(Self::map_rows(pg_rows))
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

        Ok(Self::map_rows(pg_rows))
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

    fn map_rows(pg_rows: Vec<PgRow>) -> Vec<Document> {
        pg_rows
            .iter()
            .map(|row| {
                Document::from(row.columns().iter().filter_map(|col| {
                    let ordinal = col.ordinal();
                    let type_name = row.columns()[ordinal].type_info().name().to_lowercase();

                    let value = match type_name.as_str() {
                        "json" | "jsonb" => {
                            match row.try_get::<Option<JsonValue>, _>(ordinal).ok()? {
                                None | Some(JsonValue::Null) => Some(Value::null()),
                                Some(JsonValue::String(s)) => Some(Value::string(s)),
                                Some(JsonValue::Bool(b)) => Some(Value::bool(b)),
                                Some(JsonValue::Number(n)) => {
                                    if let Some(i) = n.as_i64() {
                                        Some(Value::i64(i))
                                    } else {
                                        n.as_f64().map(Value::f64)
                                    }
                                }
                                Some(other) => Some(Value::string(other.to_string())),
                            }
                        }
                        "int2" | "int4" | "int8" => row
                            .try_get::<Option<i64>, _>(ordinal)
                            .ok()
                            .map(|value| value.map(Value::i64).unwrap_or_else(Value::null)),
                        "float4" => row
                            .try_get::<Option<f32>, _>(ordinal)
                            .ok()
                            .map(|value| value.map(Value::f32).unwrap_or_else(Value::null)),
                        "float8" => row
                            .try_get::<Option<f64>, _>(ordinal)
                            .ok()
                            .map(|value| value.map(Value::f64).unwrap_or_else(Value::null)),
                        "bool" => row
                            .try_get::<Option<bool>, _>(ordinal)
                            .ok()
                            .map(|value| value.map(Value::bool).unwrap_or_else(Value::null)),
                        "bytea" => row
                            .try_get::<Option<Vec<u8>>, _>(ordinal)
                            .ok()
                            .map(|value| value.map(Value::bytes).unwrap_or_else(Value::null)),
                        _ => row
                            .try_get::<Option<String>, _>(ordinal)
                            .ok()
                            .map(|value| value.map(Value::string).unwrap_or_else(Value::null)),
                    };

                    value.map(|value| (col.name().to_string(), value))
                }))
            })
            .collect()
    }
}
