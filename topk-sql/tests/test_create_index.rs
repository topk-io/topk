use rstest::rstest;
use topk_rs::doc;

mod common;
use common::{wait_for_index, Scope, TableScope};

#[rstest]
#[case::keyword_index(
    "CREATE INDEX ON {{table}} USING keyword_index (title) WITH (type = 'text')",
    "SELECT _id FROM {{table}} WHERE match('mockingbird', title) LIMIT 10"
)]
#[case::vector_index(
    "CREATE INDEX ON {{table}} USING vector_index (embedding) WITH (metric = 'cosine')",
    "SELECT _id FROM {{table}} ORDER BY vector_distance(embedding, f32_vector(ARRAY[0.0, 1.0, 0.0, 0.0])) LIMIT 1",
)]
#[tokio::test]
async fn create_index_on_written_data(#[case] create_sql: &str, #[case] select_sql: &str) {
    let rows = TableScope::with_scope(async |ctx| {
        ctx.sql("CREATE TABLE {{table}} (title TEXT NOT NULL, embedding f32_vector(4))")
            .await?;
        ctx.sql(
            "INSERT INTO {{table}} (_id, title, embedding)
             VALUES ('a', 'To Kill a Mockingbird', f32_vector(ARRAY[0.0, 1.0, 0.0, 0.0]))",
        )
        .await?;
        ctx.sql(create_sql).await?;
        assert_eq!(
            ctx.sql("SELECT title FROM {{table}} WHERE _id = 'a'")
                .await?,
            vec![doc!("title" => "To Kill a Mockingbird")]
        );
        wait_for_index(ctx, select_sql).await
    })
    .await
    .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id().unwrap(), "a");
}

#[tokio::test]
async fn ngram_addition_keeps_regex_readable() {
    let rows = TableScope::with_scope(async |ctx| {
        ctx.sql("CREATE TABLE {{table}} (title TEXT NOT NULL)")
            .await?;
        ctx.sql("INSERT INTO {{table}} (_id, title) VALUES ('a', 'To Kill a Mockingbird')")
            .await?;
        ctx.sql("CREATE INDEX ON {{table}} USING ngram_index (title)")
            .await?;
        ctx.sql("SELECT _id FROM {{table}} WHERE title ~ 'ockingb' LIMIT 10")
            .await
    })
    .await
    .unwrap();
    assert_eq!(rows, vec![doc!("_id" => "a")]);
}

#[tokio::test]
async fn drop_index_keeps_the_column() {
    let rows = TableScope::with_scope(async |ctx| {
        ctx.sql("CREATE TABLE {{table}} (title TEXT NOT NULL INDEX keyword_index())")
            .await?;
        ctx.sql("INSERT INTO {{table}} (_id, title) VALUES ('a', 'Dune')")
            .await?;
        ctx.sql(&format!("DROP INDEX {}_title_idx", ctx.name))
            .await?;
        ctx.sql(&format!("DROP INDEX IF EXISTS {}_missing_idx", ctx.name))
            .await?;
        ctx.sql("INSERT INTO {{table}} (_id, title) VALUES ('b', 'Emma')")
            .await?;
        ctx.sql("CREATE INDEX ON {{table}} USING keyword_index (title) WITH (type = 'text')")
            .await?;
        ctx.sql("SELECT title FROM {{table}} WHERE title = 'Dune'")
            .await
    })
    .await
    .unwrap();

    assert_eq!(rows, vec![doc!("title" => "Dune")]);
}

#[rstest]
#[case::missing_using(
    "CREATE INDEX ON {{table}} (title)",
    "CREATE INDEX requires USING <method>"
)]
#[case::unknown_method(
    "CREATE INDEX ON {{table}} USING btree_index (title)",
    "unknown index method `btree_index`"
)]
#[case::semantic_index(
    "CREATE INDEX ON {{table}} USING semantic_index (title)",
    "semantic_index on an existing collection"
)]
#[case::unknown_column(
    "CREATE INDEX ON {{table}} USING keyword_index (nope)",
    "column `nope` does not exist"
)]
#[case::already_indexed(
    "CREATE INDEX ON {{table}} USING keyword_index (indexed_title)",
    "already indexed"
)]
#[case::two_columns(
    "CREATE INDEX ON {{table}} USING keyword_index (title, indexed_title)",
    "exactly one column"
)]
#[case::operator_class(
    "CREATE INDEX ON {{table}} USING vector_index (embedding vector_cosine_ops)",
    "operator class"
)]
#[case::unknown_index("DROP INDEX {{table}}_nope_idx", "does not exist")]
#[case::custom_index_name(
    "CREATE INDEX my_index ON {{table}} USING keyword_index (title)",
    "index name must be"
)]
#[tokio::test]
async fn create_index_rejected(#[case] failing_sql: &str, #[case] expected: &str) {
    let err = TableScope::with_scope(async |ctx| -> anyhow::Result<()> {
        ctx.sql(
            "CREATE TABLE {{table}} (
                title          TEXT NOT NULL,
                indexed_title  TEXT INDEX keyword_index(),
                embedding      f32_vector(4)
            )",
        )
        .await?;
        ctx.sql(failing_sql).await?;
        Ok(())
    })
    .await
    .unwrap_err();

    assert!(
        err.to_string().contains(expected),
        "expected `{expected}` in `{err}`"
    );
}

#[tokio::test]
async fn an_index_name_two_tables_can_produce_is_ambiguous() {
    let err = TableScope::with_scope(async |ctx| -> anyhow::Result<()> {
        let other = format!("{}_a", ctx.name);
        ctx.sql("CREATE TABLE {{table}} (a_b TEXT NOT NULL INDEX keyword_index())")
            .await?;
        ctx.sql(&format!(
            "CREATE TABLE {other} (b TEXT NOT NULL INDEX keyword_index())"
        ))
        .await?;

        let result = ctx.sql(&format!("DROP INDEX {}_a_b_idx", ctx.name)).await;
        ctx.sql(&format!("DROP TABLE IF EXISTS {other}")).await?;
        result.map(|_| ())
    })
    .await
    .unwrap_err();

    assert!(
        err.to_string().contains("is ambiguous"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn an_exact_keyword_index_needs_a_column_with_no_stored_values() {
    let err = TableScope::with_scope(async |ctx| -> anyhow::Result<()> {
        ctx.sql("CREATE TABLE {{table}} (title TEXT NOT NULL)")
            .await?;
        ctx.sql("INSERT INTO {{table}} (_id, title) VALUES ('a', 'Dune')")
            .await?;
        ctx.sql("CREATE INDEX ON {{table}} USING keyword_index (title) WITH (type = 'exact')")
            .await?;
        Ok(())
    })
    .await
    .unwrap_err();

    assert!(
        err.to_string()
            .contains("stored values may be longer than 64 characters"),
        "unexpected error: {err}"
    );
}
