use futures_util::TryStreamExt;
use test_context::test_context;
use topk_rs::proto::v1::ctx::file::InputFile;
use topk_rs::proto::v1::data::Value;

mod utils;
use utils::{dataset::test_pdf_path, ProjectTestContext};

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_dataset_list(ctx: &mut ProjectTestContext) {
    let response = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    ctx.client
        .dataset(&response.dataset().unwrap().name)
        .upsert_file(
            "doc1",
            InputFile::from_path(test_pdf_path()).expect("could not create InputFile from path"),
            Vec::<(String, Value)>::new(),
        )
        .await
        .expect("could not upsert PDF file");

    let entries: Vec<_> = ctx
        .client
        .dataset(&response.dataset().unwrap().name)
        .list(None, None)
        .await
        .expect("could not list dataset entries")
        .into_inner()
        .try_collect()
        .await
        .expect("could not receive entries from stream");

    assert_eq!(entries.len(), 1);
}
