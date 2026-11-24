use futures::TryStreamExt;
use rstest::rstest;
use serde_json::json;
use test_macros::rstest_ctx;
use topk::import::plan::Target;
use topk::import::source::{Cursor, ReadEvent};
use topk_rs::proto::v1::control::FieldSpec;

use crate::import::common::seed::es;
use crate::import::common::seed::Backend::Elasticsearch;
use crate::import::common::*;

/// A query elasticsearch rejects comes back with elasticsearch's reason, not
/// just the status line — `error_for_status_code` throws that body away.
#[rstest_ctx(Scratch)]
async fn es_rejected_query_carries_the_reason(ctx: &Scratch) {
    let seeded = ctx.seed(Elasticsearch, "books", books()).await;
    let mut target = target(&seeded.from, "_id", [("title", FieldSpec::text(false))]);
    target.filter = Some(r#"{"bogus_query": {}}"#.into());
    let source = source(&seeded.url).await.unwrap();
    let error = json_documents(&source, &target)
        .await
        .unwrap_err()
        .to_string();

    // The status line does not name the query; only the body does.
    assert!(error.contains("400"), "{error}");
    assert!(error.contains("bogus_query"), "{error}");
}

#[rstest_ctx(Scratch)]
async fn elasticsearch_url_index_is_a_selector(ctx: &Scratch) {
    let seeded = ctx.seed(Elasticsearch, "books", books()).await;
    let target = target(&seeded.from, "_id", [("title", FieldSpec::text(false))]);
    let source = source(&seeded.url).await.unwrap();
    let table = source.discover().await.unwrap();
    let docs = json_documents(&source, &target).await.unwrap();

    assert_eq!(table.from, target.from);
    assert_eq!(docs.len(), 3);
}

#[rstest]
#[case::array_filter("[]")]
#[case::null_filter("null")]
#[case::scalar_filter("42")]
#[case::malformed_filter("{")]
#[tokio::test]
async fn es_filter_requires_object(#[case] filter: &str) {
    let source = source("es+http://127.0.0.1:1/books").await.unwrap();
    let target = Target {
        filter: Some(filter.to_string()),
        ..target("books", "_id", [("title", FieldSpec::text(false))])
    };

    let error = refused(source.scan(&target, None).await);

    assert!(error.contains("is not a JSON object"), "{error}");
}

#[rstest_ctx(Scratch)]
async fn expired_cursor_resets_limit(ctx: &Scratch) {
    let seeded = ctx.seed(Elasticsearch, "books", books()).await;
    let source = source(&seeded.url).await.unwrap();
    let mut target = target(&seeded.from, "_id", [("title", FieldSpec::text(false))]);
    target.limit = Some(2);
    let resumed = topk::import::source::Resume {
        open: vec![topk::import::source::Cursor::Page {
            pit: "expired".into(),
            sort: vec![1],
        }],
        done: Vec::new(),
        consumed: 1,
    };
    let mut scan = source.scan(&target, Some(&resumed)).await.unwrap();
    let mut restarted = false;
    let mut rows = 0;
    while let Some(event) = futures::TryStreamExt::try_next(&mut scan).await.unwrap() {
        match event {
            topk::import::source::ReadEvent::Restarted => {
                assert_eq!(rows, 0);
                assert!(!restarted);
                restarted = true;
            }
            topk::import::source::ReadEvent::Chunk(chunk) => {
                assert!(restarted);
                rows += chunk.rows.len();
                assert!(chunk.rows.iter().all(Result::is_ok));
            }
        }
    }
    assert!(restarted);
    assert_eq!(rows, 2);
}

#[rstest_ctx(Scratch)]
#[case::limited_page(3, Some(1), true)]
#[case::last_page(3, None, true)]
#[case::interrupted_page(1001, None, false)]
async fn pit_lifetime(
    ctx: &Scratch,
    #[case] count: u64,
    #[case] limit: Option<u64>,
    #[case] closed: bool,
) {
    let seeded = ctx.seed(Elasticsearch, "pit", rows(1, count)).await;
    let source = source(&seeded.url).await.unwrap();
    let mut target = target(&seeded.from, "_id", [("name", FieldSpec::text(false))]);
    target.limit = limit;
    let mut scan = source.scan(&target, None).await.unwrap();
    let pit = match scan.try_next().await.unwrap().unwrap() {
        ReadEvent::Chunk(chunk) => {
            assert_eq!(chunk.rows.len(), limit.unwrap_or(count.min(1000)) as usize);
            match chunk.cursor.unwrap() {
                Cursor::Page { pit, .. } => pit,
                other => panic!("unexpected cursor {other:?}"),
            }
        }
        ReadEvent::Restarted => panic!("unexpected restart"),
    };
    drop(scan);
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:19200/_search")
        .json(&json!({"size": 0, "pit": {"id": pit, "keep_alive": "1m"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), if closed { 404 } else { 200 });
    if !closed {
        client
            .delete("http://localhost:19200/_pit")
            .json(&json!({"id": pit}))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }
}

#[rstest_ctx(Scratch)]
async fn aliases_resolve_one_index(ctx: &Scratch) {
    let first = ctx.seed(Elasticsearch, "first", books()).await;
    let second = ctx.seed(Elasticsearch, "second", books()).await;
    let alias = ctx.name("alias");
    let client = reqwest::Client::new();
    client
        .post("http://localhost:19200/_aliases")
        .json(&json!({"actions":[{"add":{"index":first.from,"alias":alias}}]}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let selected = es::source(&alias).await.unwrap();
    let target = Target::new(selected.discover().await.unwrap(), None).unwrap();
    assert_eq!(target.from.value(), first.from);
    assert_eq!(json_documents(&selected, &target).await.unwrap().len(), 3);
    client
        .post("http://localhost:19200/_aliases")
        .json(&json!({"actions":[{"add":{"index":second.from,"alias":alias}}]}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let selected = es::source(&alias).await.unwrap();
    assert!(refused(selected.discover().await).contains("resolves to 2 indices"));
    let missing = es::source(&ctx.name("missing")).await.unwrap();
    assert!(missing.discover().await.is_err());
}

#[rstest_ctx(Scratch)]
async fn unreadable_selected_mapping_fails(ctx: &Scratch) {
    let index = ctx.name("unsupported");
    reqwest::Client::new()
        .put(format!("http://localhost:19200/{index}"))
        .json(&json!({"mappings":{"properties":{"rank":{"type":"rank_features"}}}}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let selected = es::source(&index).await.unwrap();
    assert!(refused(selected.discover().await).contains("rank_features"));
}
