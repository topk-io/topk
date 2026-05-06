use futures_util::TryStreamExt;
use test_context::test_context;

use topk_rs::proto::v1::data::Value;
use topk_rs::{Client, ClientConfig, Error};

mod utils;
use utils::ProjectTestContext;

use crate::utils::dataset::test_pdf;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_search(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None)
        .await
        .expect("could not create dataset");

    let handle = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file("doc1", test_pdf(), Vec::<(String, Value)>::new())
        .await
        .expect("could not upsert file");

    ctx.client
        .dataset(&dataset.name)
        .wait_for_handle(&handle, None)
        .await
        .expect("could not wait for handle");

    let results: Vec<_> = ctx
        .client
        .search(
            "What is bootstraping in programming language design?",
            [&dataset.name],
            5,
            None,
            Vec::<String>::new(),
        )
        .await
        .expect("could not call search")
        .try_collect()
        .await
        .expect("could not collect search results");

    assert!(!results.is_empty(), "expected at least one search result");
    assert!(results.iter().all(|r| r.doc_id == "doc1"));
}

#[tokio::test]
async fn test_search_empty_datasets() {
    let err = Client::new(ClientConfig::new("dummy-key", "us-east-1"))
        .search("query", Vec::<&str>::new(), 10, None, Vec::<String>::new())
        .await
        .expect_err("should fail with empty datasets");

    assert!(
        matches!(err, Error::InvalidArgument(ref s) if s == "provide at least one dataset"),
        "unexpected error: {err}"
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_search_select_internal_fields(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None)
        .await
        .expect("could not create dataset");

    let handle = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file("doc1", test_pdf(), [("title", "Test PDF")])
        .await
        .expect("could not upsert file");

    ctx.client
        .dataset(&dataset.name)
        .wait_for_handle(&handle, None)
        .await
        .expect("could not wait for handle");

    let results: Vec<_> = ctx
        .client
        .search(
            "What is bootstraping in programming language design?",
            [&dataset.name],
            5,
            None,
            ["topk.name", "topk.size", "topk.mime_type", "title"],
        )
        .await
        .expect("could not call search")
        .try_collect()
        .await
        .expect("could not collect search results");

    assert!(!results.is_empty(), "expected at least one search result");

    for result in &results {
        assert_eq!(
            result.metadata.get("topk.name").and_then(|v| v.as_string()),
            Some("pdfko.pdf"),
            "expected topk.name in metadata: {:?}",
            result.metadata
        );
        assert_eq!(
            result
                .metadata
                .get("topk.mime_type")
                .and_then(|v| v.as_string()),
            Some("application/pdf"),
        );
        assert!(
            result
                .metadata
                .get("topk.size")
                .and_then(|v| v.as_u64())
                .is_some(),
            "expected topk.size in metadata: {:?}",
            result.metadata
        );
        assert_eq!(
            result.metadata.get("title").and_then(|v| v.as_string()),
            Some("Test PDF"),
        );
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_search_rejects_non_selectable_internal_field(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None)
        .await
        .expect("could not create dataset");

    let err = ctx
        .client
        .search("query", [&dataset.name], 5, None, ["topk.score"])
        .await
        .expect_err("should reject non-selectable internal field");

    assert!(
        matches!(err, Error::InvalidArgument(ref s) if s == "Invalid selected field: topk.score"),
        "unexpected error: {err}"
    );
}
