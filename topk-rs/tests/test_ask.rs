use futures_util::StreamExt;
use test_context::test_context;

use topk_rs::proto::v1::{
    ctx::{ask_result::Message, AskResult},
    data::Value,
};

mod utils;
use utils::ProjectTestContext;

use crate::utils::dataset::{test_pdf, quick_wait};

#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore]
async fn test_ask(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
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

    // Wait for file to be processed
    ctx.client
        .dataset(&dataset.name)
        .wait_for_handle(&upsert.handle, quick_wait())
        .await
        .expect("could not wait handle");

    // Ask
    let mut stream = ctx
        .client
        .ask(
            "What score must general education students achieve who first entered ninth grade in 1997 ?",
            [&dataset.name],
            None,
            None,
            None
        )
        .await
        .expect("could not call ask");

    let mut last_message: Option<AskResult> = None;
    while let Some(result) = stream.next().await {
        last_message = Some(result.expect("could not receive message from stream"));
    }

    assert!(matches!(
        last_message,
        Some(AskResult {
            message: Some(Message::Answer(_))
        })
    ));
}
