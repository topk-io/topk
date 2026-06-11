use rstest::rstest;

mod common;
use common::{BooksContext, Scope};

#[rstest]
#[case::select(
    "EXPLAIN SELECT _id FROM {{table}} WHERE author = 'Tolkien' LIMIT 5",
    &["Query", "Collection"],
)]
#[case::verbose_select(
    "EXPLAIN VERBOSE SELECT _id FROM {{table}} WHERE author = 'Tolkien' LIMIT 5",
    &["\n"],
)]
#[case::insert(
    "EXPLAIN INSERT INTO {{table}} (_id, title, author, published_year, rating, genre, in_print) VALUES ('x','X','X',2000,4.0,'fiction',true)",
    &["Insert"],
)]
#[case::update(
    "EXPLAIN UPDATE {{table}} SET rating = 5.0 WHERE _id = 'hobbit'",
    &["Update"],
)]
#[case::delete(
    "EXPLAIN DELETE FROM {{table}} WHERE _id = 'hobbit'",
    &["Delete"],
)]
#[tokio::test]
async fn explain(#[case] query: &str, #[case] fragments: &[&str]) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();

    let plan = rows
        .first()
        .and_then(|doc| doc.fields.get("plan"))
        .and_then(|value| value.as_string())
        .unwrap();
    for fragment in fragments {
        assert!(
            plan.contains(fragment),
            "expected plan to contain `{fragment}`, got: {plan}"
        );
    }
}
