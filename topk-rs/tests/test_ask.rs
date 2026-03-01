use std::time::Duration;

use futures_util::StreamExt;
use test_context::test_context;

use topk_rs::proto::v1::{
    ctx::{ask_result::Message, file::InputFile, AskResult},
    data::Value,
};

mod utils;
use utils::{dataset::test_pdf_path, ProjectTestContext};

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_ask(ctx: &mut ProjectTestContext) {
    let create = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let upsert = ctx
        .client
        .dataset(&create.dataset().unwrap().name)
        .upsert_file(
            "doc1",
            InputFile::from_path(test_pdf_path()).expect("could not create InputFile from path"),
            Vec::<(String, Value)>::new(),
        )
        .await
        .expect("could not upsert file");

    let max_attempts = 120;
    for _ in 0..max_attempts {
        let check_handle = ctx
            .client
            .dataset(&create.dataset().unwrap().name)
            .check_handle(upsert.handle.clone().into())
            .await
            .expect("could not check handle");

        if check_handle.processed {
            break;
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    // assert!(
    //     processed,
    //     "Handle was not processed within {} seconds",
    //     max_attempts
    // );

    let mut stream = ctx
        .client
        .ask(
            "What score must general education students achieve who first entered ninth grade in 1997 ?",
            [&create.dataset().unwrap().name],
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
