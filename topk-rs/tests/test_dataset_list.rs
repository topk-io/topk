use futures_util::TryStreamExt;
use test_context::test_context;
use topk_rs::proto::v1::data::Value;

mod utils;
use utils::{dataset::test_pdf, ProjectTestContext};

#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore]
async fn test_dataset_list(ctx: &mut ProjectTestContext) {
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
        .expect("could not upsert PDF file");

    ctx.client
        .dataset(&dataset.name)
        .wait_for_handle(&handle, None)
        .await
        .expect("could not wait for handle");

    let entries: Vec<_> = ctx
        .client
        .dataset(&dataset.name)
        .list(None, None)
        .await
        .expect("could not list dataset entries")
        .try_collect()
        .await
        .expect("could not receive entries from stream");

    assert_eq!(entries.len(), 1);
}
