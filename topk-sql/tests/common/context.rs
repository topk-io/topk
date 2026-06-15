#![allow(dead_code)]

use test_context::AsyncTestContext;
use topk_rs::proto::v1::data::{Document, Value};
use uuid::Uuid;

use super::client::SqlClient;

#[allow(async_fn_in_trait)]
pub trait Scope: AsyncTestContext + Sized {
    async fn with_scope<F, R>(f: F) -> R
    where
        F: AsyncFnOnce(&Self) -> R,
    {
        let ctx = Self::setup().await;
        let result = f(&ctx).await;
        ctx.teardown().await;
        result
    }
}

impl<C: AsyncTestContext> Scope for C {}

pub struct TableScope {
    pub sql: SqlClient,
    pub name: String,
}

impl TableScope {
    pub async fn sql(&self, sql: &str) -> anyhow::Result<Vec<Document>> {
        let sql = sql.replace("{{table}}", &self.name);
        self.sql.query(&sql).await
    }
}

impl AsyncTestContext for TableScope {
    async fn setup() -> Self {
        let name = format!("pgwire_{}", Uuid::new_v4().simple());
        let sql = SqlClient::from_env().await.unwrap();
        Self { sql, name }
    }

    async fn teardown(self) {
        let _ = self
            .sql
            .query(&format!("DROP TABLE IF EXISTS {}", self.name))
            .await;
    }
}

pub struct BooksContext {
    inner: TableScope,
}

impl BooksContext {
    pub fn table(&self) -> &str {
        &self.inner.name
    }

    pub async fn sql(&self, sql: impl Into<String>) -> anyhow::Result<Vec<Document>> {
        let sql = sql.into().replace("{{table}}", &self.inner.name);
        self.inner.sql.query(&sql).await
    }

    pub async fn column_types(
        &self,
        sql: impl Into<String>,
    ) -> anyhow::Result<Vec<(String, String)>> {
        let sql = sql.into().replace("{{table}}", &self.inner.name);
        self.inner.sql.column_types(&sql).await
    }

    pub async fn column_type(
        &self,
        sql: impl Into<String>,
        column: &str,
    ) -> anyhow::Result<String> {
        let types = self.column_types(sql).await?;
        Ok(types
            .into_iter()
            .find(|(name, _)| name == column)
            .map(|(_, ty)| ty)
            .unwrap_or_else(|| "(missing)".to_string()))
    }

    pub async fn prepared(
        &self,
        sql: impl Into<String>,
        params: Vec<Value>,
    ) -> anyhow::Result<Vec<Document>> {
        let sql = sql.into().replace("{{table}}", &self.inner.name);
        self.inner.sql.prepared(&sql, params).await
    }

    pub async fn prepared_column_types(
        &self,
        sql: impl Into<String>,
        params: Vec<Value>,
    ) -> anyhow::Result<Vec<(String, String)>> {
        let sql = sql.into().replace("{{table}}", &self.inner.name);
        self.inner.sql.prepared_column_types(&sql, params).await
    }

    pub async fn prepared_column_type(
        &self,
        sql: impl Into<String>,
        params: Vec<Value>,
        column: &str,
    ) -> anyhow::Result<String> {
        let types = self.prepared_column_types(sql, params).await?;
        Ok(types
            .into_iter()
            .find(|(name, _)| name == column)
            .map(|(_, ty)| ty)
            .unwrap_or_else(|| "(missing)".to_string()))
    }
}

pub struct SessionContext {
    sql: SqlClient,
}

impl SessionContext {
    pub async fn sql(&self, sql: &str) -> anyhow::Result<Vec<Document>> {
        self.sql.query(sql).await
    }

    pub async fn batch(&self, sqls: &[&str]) -> anyhow::Result<Vec<Document>> {
        self.sql.batch(sqls).await
    }
}

impl AsyncTestContext for SessionContext {
    async fn setup() -> Self {
        let sql = SqlClient::from_env().await.unwrap();
        Self { sql }
    }

    async fn teardown(self) {}
}

impl AsyncTestContext for BooksContext {
    async fn setup() -> Self {
        let inner = TableScope::setup().await;
        let table = &inner.name;

        inner
            .sql
            .query(&format!(
                r#"
            CREATE TABLE {table} (
                title          TEXT NOT NULL             INDEX keyword_index(),
                author         TEXT,
                published_year INTEGER,
                rating         FLOAT,
                genre          TEXT,
                in_print       BOOLEAN,
                bio            TEXT                      INDEX semantic_index(),
                embedding      f32_vector(4)             INDEX vector_index(metric = 'cosine'),
                sparse_emb     f32_sparse_vector         INDEX vector_index(metric = 'dot_product'),
                multi_emb      f32_matrix(4)             INDEX multi_vector_index(metric = 'maxsim'),
                tags           TEXT[]                    INDEX keyword_index(),
                checksum       BYTEA,
                metadata       JSONB
            );
            "#
            ))
            .await
            .expect("create books table");

        inner
            .sql
            .query(&format!(
                r#"
            INSERT INTO {table} (
                _id, title, author, published_year, rating, genre, in_print, bio, embedding, tags,
                checksum, metadata, sparse_emb, multi_emb
            )
            VALUES
                (
                    'mockingbird', 'To Kill a Mockingbird', 'Lee', 1960, 4.3, 'fiction', true,
                    'A story of racial injustice and moral courage in the American South',
                    f32_vector(ARRAY[0.0, 1.0, 0.0, 0.0]),
                    ARRAY['classic', 'american', 'fiction'],
                    bytes('deadbeef'),
                    struct('publisher', 'HarperCollins', 'pages', 281),
                    NULL, NULL
                ),
                (
                    'nineteen_eighty_four', '1984', 'Orwell', 1949, 4.2, 'dystopian', true,
                    'A dystopian tale of totalitarian surveillance and doublethink',
                    f32_vector(ARRAY[0.0, 0.0, 1.0, 0.0]),
                    ARRAY['dystopia', 'classic', 'political'],
                    bytes('cafebabe'),
                    struct('publisher', 'Secker & Warburg', 'pages', 328),
                    NULL, NULL
                ),
                (
                    'pride', 'Pride and Prejudice', 'Austen', 1813, 4.3, 'romance', true,
                    'A romantic comedy of manners among the English gentry',
                    f32_vector(ARRAY[0.0, 0.0, 0.0, 1.0]),
                    ARRAY['romance', 'classic', 'british'],
                    bytes('abcdef01'),
                    struct('publisher', 'T. Egerton', 'pages', 432),
                    NULL, NULL
                ),
                (
                    'gatsby', 'The Great Gatsby', 'Fitzgerald', 1925, 3.9, 'fiction', true,
                    'The decadence and empty ambition of the Jazz Age in America',
                    f32_vector(ARRAY[0.0, 1.0, 0.0, 0.0]),
                    ARRAY['classic', 'american', 'jazz'],
                    bytes('12345678'),
                    struct('publisher', 'Scribner', 'pages', 180),
                    NULL, NULL
                ),
                (
                    'catcher', 'The Catcher in the Rye', 'Salinger', 1951, 3.8, 'fiction', false,
                    'A teenager''s alienation and cynicism navigating New York City',
                    f32_vector(ARRAY[0.0, 1.0, 0.0, 0.0]),
                    ARRAY['coming_of_age', 'american', 'fiction'],
                    NULL, NULL, NULL, NULL
                ),
                (
                    'hobbit', 'The Hobbit', 'Tolkien', 1937, 4.3, 'fantasy', true,
                    'A hobbit joins a dwarf company on a dragon-slaying quest through Middle-earth',
                    f32_vector(ARRAY[1.0, 0.0, 0.0, 0.0]),
                    ARRAY['fantasy', 'adventure', 'tolkien'],
                    bytes('9abcdef0'),
                    NULL,
                    f32_sparse_vector(ARRAY[0, 1], ARRAY[1.0, 0.5]),
                    f32_matrix(ARRAY[ARRAY[1.0, 0.0, 0.0, 0.0], ARRAY[0.9, 0.1, 0.0, 0.0]])
                ),
                (
                    'lotr', 'The Lord of the Rings', 'Tolkien', 1954, 4.5, 'fantasy', true,
                    'An epic fantasy fellowship races to destroy a ring of ultimate power',
                    f32_vector(ARRAY[1.0, 0.0, 0.0, 0.0]),
                    ARRAY['fantasy', 'epic', 'tolkien'],
                    NULL, NULL,
                    f32_sparse_vector(ARRAY[0, 2], ARRAY[0.8, 0.3]),
                    f32_matrix(ARRAY[ARRAY[1.0, 0.0, 0.0, 0.0], ARRAY[0.8, 0.2, 0.0, 0.0]])
                ),
                (
                    'harry', 'Harry Potter and the Sorcerer''s Stone', 'Rowling', 1997, 4.5, 'fantasy', true,
                    'A young wizard discovers his magical heritage at a school for witches and wizards',
                    f32_vector(ARRAY[1.0, 0.0, 0.0, 0.0]),
                    ARRAY['fantasy', 'magic', 'children'],
                    NULL, NULL,
                    f32_sparse_vector(ARRAY[0, 3], ARRAY[0.9, 0.4]),
                    f32_matrix(ARRAY[ARRAY[1.0, 0.0, 0.0, 0.0], ARRAY[0.9, 0.0, 0.1, 0.0]])
                ),
                (
                    'alchemist', 'The Alchemist', 'Coelho', 1988, 3.9, 'fiction', true,
                    'A shepherd''s philosophical journey across the desert to find his personal legend',
                    f32_vector(ARRAY[0.0, 1.0, 0.0, 0.0]),
                    ARRAY['philosophy', 'fiction', 'spiritual'],
                    NULL, NULL, NULL, NULL
                ),
                (
                    'moby', 'Moby Dick', 'Melville', 1851, 3.5, 'adventure', false,
                    'A captain''s obsessive quest to hunt and kill a legendary white whale',
                    f32_vector(ARRAY[0.0, 0.5, 0.5, 0.0]),
                    ARRAY['classic', 'adventure', 'sea'],
                    NULL, NULL, NULL, NULL
                )
            "#
            ))
            .await
            .expect("seed books");

        Self { inner }
    }

    async fn teardown(self) {
        self.inner.teardown().await;
    }
}
