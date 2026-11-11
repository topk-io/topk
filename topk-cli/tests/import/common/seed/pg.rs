use duckdb::Connection;
use sqlx::{Executor, PgPool};
use topk_rs::proto::v1::data::Document;

use super::sql;

pub struct Pg {
    pub conn: Connection,
}

impl Pg {
    pub const URL: &str = "postgres://postgres:postgres@localhost:5433/demo?sslmode=disable";

    pub fn new() -> anyhow::Result<Pg> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("INSTALL postgres; LOAD postgres;")?;
        conn.execute_batch(&format!("ATTACH '{}' AS p (TYPE postgres);", Self::URL))?;
        Ok(Pg { conn })
    }

    /// Tables dropped when the guard goes out of scope, panic or not.
    pub fn temp(tables: &[(String, &str)]) -> Temp {
        let pg = Pg::new().expect("attach postgres");
        for (name, columns) in tables {
            pg.conn
                .execute_batch(&format!("CREATE TABLE p.{name} {columns};"))
                .expect("create table");
        }
        Temp {
            pg,
            names: tables.iter().map(|(name, _)| name.clone()).collect(),
        }
    }

    pub async fn seed(name: &str, docs: Vec<Document>) -> anyhow::Result<String> {
        let table = format!("public.{name}");
        let pool = PgPool::connect(Self::URL).await?;
        pool.execute(sql::ddl(&table, &docs, &sql::POSTGRES).as_str())
            .await?;
        pool.execute(sql::insert(&table, &docs).as_str()).await?;
        Ok(table)
    }
}

pub struct Temp {
    pg: Pg,
    names: Vec<String>,
}

impl Drop for Temp {
    fn drop(&mut self) {
        for name in &self.names {
            let _ = self
                .pg
                .conn
                .execute_batch(&format!("DROP TABLE IF EXISTS p.{name};"));
        }
    }
}
