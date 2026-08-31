use duckdb::Connection;
use sqlx::{Executor, PgPool};
use topk::import::Target;
use topk_rs::proto::v1::data::Document;
use uuid::Uuid;

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

    // Discover-only fixtures shaped by hand, not through `Seed`. Left in the
    // local Postgres afterwards — a random suffix keeps them from colliding,
    // and it's an ephemeral dev database.
    fn table(prefix: &str, columns: &str) -> anyhow::Result<String> {
        let pg = Pg::new()?;
        let stream = format!(
            "public.{prefix}_{}",
            &Uuid::new_v4().simple().to_string()[..8]
        );
        pg.conn
            .execute_batch(&format!("CREATE TABLE p.{stream} {columns};"))?;
        Ok(stream)
    }

    pub fn seed() -> String {
        Pg::table("books", "(id INTEGER PRIMARY KEY, title VARCHAR)").expect("seed postgres")
    }

    pub fn seed_keyed_on(key: &str) -> String {
        Pg::table("keyed", &format!("({key} TEXT PRIMARY KEY, title VARCHAR)"))
            .expect("seed postgres")
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

    pub fn seed_composite() -> String {
        Pg::table(
            "comp",
            "(a TEXT, b INTEGER, title VARCHAR, PRIMARY KEY (a, b))",
        )
        .expect("seed postgres")
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

#[async_trait::async_trait(?Send)]
impl super::Seed for Pg {
    async fn seed(&self, name: &str, docs: Vec<Document>) -> anyhow::Result<Target> {
        let table = format!("public.{name}");
        let pool = PgPool::connect(Self::URL).await?;
        pool.execute(sql::ddl(&table, &docs, &sql::POSTGRES).as_str())
            .await?;
        pool.execute(sql::insert(&table, &docs).as_str()).await?;
        Ok(super::discovered(
            Target {
                from: table,
                ..Default::default()
            },
            self.url(),
        )
        .await?)
    }

    fn url(&self) -> Option<String> {
        Some(Self::URL.to_string())
    }
}
