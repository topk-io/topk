use futures_util::TryStreamExt;
use test_context::test_context;

use topk_rs::proto::v1::data::Value;
use topk_rs::{Client, ClientConfig, Error};

mod utils;
use utils::ProjectTestContext;

use crate::utils::dataset::test_pdf;

#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore]
async fn test_search(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None)
        .await
        .expect("could not create dataset")
        .into_inner()
        .dataset
        .unwrap();

    let upsert = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file("doc1", test_pdf(), Vec::<(String, Value)>::new())
        .await
        .expect("could not upsert file");

    ctx.client
        .dataset(&dataset.name)
        .wait_for_handle(&upsert.handle, None)
        .await
        .expect("could not wait for handle");

    let results: Vec<_> = ctx
        .client
        .search(
            "What score must students achieve?",
            [&dataset.name],
            5,
            None,
            Vec::<String>::new(),
        )
        .await
        .expect("could not call search")
        .into_inner()
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
