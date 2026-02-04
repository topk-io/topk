use std::collections::HashMap;
use std::time::Duration;

use futures_util::StreamExt;
use test_context::test_context;

use topk_rs::{
    client::AskExt,
    proto::v1::ctx::{ask_response_message, file::InputFile, AskResponseMessage, Effort, Source},
};

mod utils;
use utils::{dataset::test_pdf_path, ProjectTestContext};

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_ask(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let handle = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file(
            "doc1".to_string(),
            InputFile::from_path(test_pdf_path()).expect("could not create InputFile from path"),
            HashMap::default(),
        )
        .await
        .expect("could not upsert file");

    let max_attempts = 120;
    let mut processed = false;
    for _ in 0..max_attempts {
        processed = ctx
            .client
            .dataset(&dataset.name)
            .check_handle(handle.clone())
            .await
            .expect("could not check handle");

        if processed {
            break;
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    assert!(
        processed,
        "Handle was not processed within {} seconds",
        max_attempts
    );

    let sources = vec![Source {
        dataset: dataset.name.clone(),
        filter: None,
    }];

    let mut stream = ctx
        .client
        .ask(
            "What score must general education students achieve who first entered ninth grade in 1997 ?".to_string(),
            sources,
            None,
            Effort::Medium,
        )
        .await
        .expect("could not call ask");

    let mut last_message: Option<AskResponseMessage> = None;

    while let Some(result) = stream.next().await {
        last_message = Some(result.expect("could not receive message from stream"));
    }

    assert!(matches!(
        last_message,
        Some(AskResponseMessage {
            message: Some(ask_response_message::Message::FinalAnswer(_))
        })
    ));
}
