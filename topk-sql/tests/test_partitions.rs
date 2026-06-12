use rstest::rstest;
use topk_rs::doc;

mod common;
use common::{BooksContext, Scope};

// Covers all [schema.]collection[$partition] syntax variants.
#[rstest]
#[case::collection(
    "SELECT title FROM {{table}} WHERE _id = 'a'",
    "default"
)]
#[case::schema_collection(
    "SELECT title FROM public.{{table}} WHERE _id = 'a'",
    "default"
)]
#[case::dollar(
    "SELECT title FROM {{table}}$p1 WHERE _id = 'a'",
    "p1"
)]
#[case::schema_dollar(
    "SELECT title FROM public.{{table}}$p1 WHERE _id = 'a'",
    "p1"
)]
#[case::partition_keyword(
    "SELECT title FROM {{table}} PARTITION p1 WHERE _id = 'a'",
    "p1"
)]
#[case::schema_partition_keyword(
    "SELECT title FROM public.{{table}} PARTITION p1 WHERE _id = 'a'",
    "p1"
)]
#[tokio::test]
async fn syntax(#[case] select_sql: &str, #[case] expected_title: &str) {
    BooksContext::with_scope(async move |ctx| {
        ctx.sql("INSERT INTO {{table}} (_id, title) VALUES ('a', 'default')")
            .await?;
        ctx.sql("INSERT INTO {{table}}$p1 (_id, title) VALUES ('a', 'p1')")
            .await?;

        let rows = ctx.sql(select_sql).await?;
        assert_eq!(rows, vec![doc!("title" => expected_title)]);

        Ok::<_, anyhow::Error>(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn insert_isolation() {
    BooksContext::with_scope(async |ctx| {
        ctx.sql("INSERT INTO {{table}} (_id, title) VALUES ('a', 'default')")
            .await?;
        ctx.sql("INSERT INTO {{table}}$p1 (_id, title) VALUES ('a', 'p1')")
            .await?;

        let default = ctx
            .sql("SELECT title FROM {{table}} WHERE _id = 'a'")
            .await?;
        let p1 = ctx
            .sql("SELECT title FROM {{table}}$p1 WHERE _id = 'a'")
            .await?;

        assert_eq!(default, vec![doc!("title" => "default")]);
        assert_eq!(p1, vec![doc!("title" => "p1")]);

        Ok::<_, anyhow::Error>(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn same_id_two_partitions() {
    BooksContext::with_scope(async |ctx| {
        ctx.sql("INSERT INTO {{table}}$p1 (_id, title) VALUES ('x', 'from p1')")
            .await?;
        ctx.sql("INSERT INTO {{table}}$p2 (_id, title) VALUES ('x', 'from p2')")
            .await?;

        let p1 = ctx
            .sql("SELECT title FROM {{table}}$p1 WHERE _id = 'x'")
            .await?;
        let p2 = ctx
            .sql("SELECT title FROM {{table}}$p2 WHERE _id = 'x'")
            .await?;

        assert_eq!(p1, vec![doc!("title" => "from p1")]);
        assert_eq!(p2, vec![doc!("title" => "from p2")]);

        Ok::<_, anyhow::Error>(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn update_isolation() {
    BooksContext::with_scope(async |ctx| {
        ctx.sql("INSERT INTO {{table}} (_id, title) VALUES ('x', 'default')")
            .await?;
        ctx.sql("INSERT INTO {{table}}$p1 (_id, title) VALUES ('x', 'p1')")
            .await?;

        ctx.sql("UPDATE {{table}}$p1 SET title = 'updated' WHERE _id = 'x'")
            .await?;

        let default = ctx
            .sql("SELECT title FROM {{table}} WHERE _id = 'x'")
            .await?;
        let p1 = ctx
            .sql("SELECT title FROM {{table}}$p1 WHERE _id = 'x'")
            .await?;

        assert_eq!(default, vec![doc!("title" => "default")]);
        assert_eq!(p1, vec![doc!("title" => "updated")]);

        Ok::<_, anyhow::Error>(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn delete_document_isolation() {
    BooksContext::with_scope(async |ctx| {
        ctx.sql("INSERT INTO {{table}} (_id, title) VALUES ('x', 'default')")
            .await?;
        ctx.sql("INSERT INTO {{table}}$p1 (_id, title) VALUES ('x', 'p1')")
            .await?;
        ctx.sql("INSERT INTO {{table}}$p1 (_id, title) VALUES ('y', 'p1-other')")
            .await?;

        ctx.sql("DELETE FROM {{table}}$p1 WHERE _id = 'x'").await?;

        let p1_x = ctx
            .sql("SELECT title FROM {{table}}$p1 WHERE _id = 'x'")
            .await?;
        assert!(p1_x.is_empty());

        let p1_y = ctx
            .sql("SELECT title FROM {{table}}$p1 WHERE _id = 'y'")
            .await?;
        assert_eq!(p1_y, vec![doc!("title" => "p1-other")]);

        let default_x = ctx
            .sql("SELECT title FROM {{table}} WHERE _id = 'x'")
            .await?;
        assert_eq!(default_x, vec![doc!("title" => "default")]);

        Ok::<_, anyhow::Error>(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn delete_partition() {
    BooksContext::with_scope(async |ctx| {
        ctx.sql("INSERT INTO {{table}} (_id, title) VALUES ('x', 'default')")
            .await?;
        ctx.sql("INSERT INTO {{table}}$p1 (_id, title) VALUES ('a', 'p1-a')")
            .await?;
        ctx.sql("INSERT INTO {{table}}$p1 (_id, title) VALUES ('b', 'p1-b')")
            .await?;
        ctx.sql("INSERT INTO {{table}}$p2 (_id, title) VALUES ('a', 'p2-a')")
            .await?;

        ctx.sql("DELETE FROM {{table}}$p1").await?;

        let p2 = ctx
            .sql("SELECT title FROM {{table}}$p2 WHERE _id = 'a'")
            .await?;
        assert_eq!(p2, vec![doc!("title" => "p2-a")]);

        let default = ctx
            .sql("SELECT title FROM {{table}} WHERE _id = 'x'")
            .await?;
        assert_eq!(default, vec![doc!("title" => "default")]);

        Ok::<_, anyhow::Error>(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn count_partition_scoped() {
    BooksContext::with_scope(async |ctx| {
        ctx.sql("INSERT INTO {{table}}$p1 (_id, title) VALUES ('a', 'one')")
            .await?;
        ctx.sql("INSERT INTO {{table}}$p1 (_id, title) VALUES ('b', 'two')")
            .await?;
        ctx.sql("INSERT INTO {{table}}$p2 (_id, title) VALUES ('a', 'other')")
            .await?;

        let p1 = ctx.sql("SELECT COUNT(*) FROM {{table}}$p1").await?;
        let p2 = ctx.sql("SELECT COUNT(*) FROM {{table}}$p2").await?;

        assert_eq!(p1, vec![doc!("_count" => 2_i64)]);
        assert_eq!(p2, vec![doc!("_count" => 1_i64)]);

        Ok::<_, anyhow::Error>(())
    })
    .await
    .unwrap();
}

#[rstest]
#[case::delete_without_filter(
    "DELETE FROM {{table}}",
    "Invalid: DELETE without a WHERE clause is not allowed"
)]
#[case::non_existent_partition(
    "SELECT title FROM {{table}}$missing LIMIT 1",
    "Partition does not exist"
)]
#[case::too_many_parts(
    "SELECT _id FROM {{table}}.p1.extra",
    "Invalid: invalid table reference; supported forms: collection, schema.collection, collection$partition, schema.collection$partition, collection PARTITION name"
)]
#[tokio::test]
async fn rejected(#[case] query: &str, #[case] expected: &str) {
    let err = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap_err();

    assert_eq!(err.to_string(), expected);
}
