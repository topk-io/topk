use topk_rs::doc;
use topk_rs::proto::v1::data::Value;

mod common;
use common::{BooksContext, Scope, TableScope};

#[tokio::test]
async fn describe() {
    BooksContext::with_scope(async |ctx| {
        let rows = ctx.sql("DESCRIBE {{table}}").await?;

        let column = |name: &str, data_type: &str, nullable: bool, index: Option<&str>| {
            doc!(
                "column_name" => name,
                "data_type" => data_type,
                "nullable" => nullable,
                "index" => index.map_or_else(Value::null, Value::string)
            )
        };
        assert_eq!(
            rows,
            vec![
                column("_id", "TEXT", false, None),
                column("author", "TEXT", true, None),
                column("bio", "TEXT", true, Some("semantic_index()")),
                column("checksum", "BYTEA", true, None),
                column(
                    "embedding",
                    "f32_vector(4)",
                    true,
                    Some("vector_index(metric = 'cosine')")
                ),
                column("genre", "TEXT", true, None),
                column("in_print", "BOOLEAN", true, None),
                column("metadata", "JSONB", true, None),
                // the options the server filled in when it created the index
                column(
                    "multi_emb",
                    "f32_matrix(4)",
                    true,
                    Some("multi_vector_index(metric = 'maxsim', quantization = 'scalar', width = '32768', top_k = '32')")
                ),
                column("nullable_importance", "FLOAT", true, None),
                column("published_ts", "TIMESTAMP", true, None),
                column("published_year", "INTEGER", true, None),
                column("rating", "FLOAT", true, None),
                column(
                    "sparse_emb",
                    "f32_sparse_vector",
                    true,
                    Some("vector_index(metric = 'dot_product')")
                ),
                column("tags", "TEXT[]", true, Some("keyword_index()")),
                column("title", "TEXT", false, Some("keyword_index()")),
            ]
        );

        Ok::<_, anyhow::Error>(())
    })
    .await
    .unwrap();
}

// A schemaless collection still has `_id`.
#[tokio::test]
async fn describe_schemaless() {
    TableScope::with_scope(async |ctx| {
        ctx.sql("CREATE TABLE {{table}} ()").await?;
        ctx.sql("INSERT INTO {{table}} (_id, title) VALUES ('a', 'x')")
            .await?;

        let rows = ctx.sql("DESCRIBE {{table}}").await?;
        assert_eq!(
            rows,
            vec![doc!(
                "column_name" => "_id",
                "data_type" => "TEXT",
                "nullable" => false,
                "index" => Value::null()
            )]
        );

        Ok::<_, anyhow::Error>(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn describe_missing() {
    let err = TableScope::with_scope(async |ctx| ctx.sql("DESCRIBE {{table}}").await)
        .await
        .unwrap_err();

    assert_eq!(err.to_string(), "Table does not exist");
}

// Drivers prepare statements and read the rows in binary.
#[tokio::test]
async fn describe_prepared() {
    BooksContext::with_scope(async |ctx| {
        let prepared = ctx.prepared("DESCRIBE {{table}}", vec![]).await?;
        assert_eq!(prepared, ctx.sql("DESCRIBE {{table}}").await?);

        Ok::<_, anyhow::Error>(())
    })
    .await
    .unwrap();
}
