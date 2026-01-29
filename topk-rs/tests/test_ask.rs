use std::collections::HashMap;
use std::time::Duration;

use futures_util::StreamExt;
use test_context::test_context;

use topk_rs::{
    client::AskExt,
    proto::v1::ctx::{ask_response_message, Effort, Source},
};

mod utils;
use utils::{dataset::test_pdf_path, ProjectTestContext};

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_ask_basic(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let handle = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file("doc1".to_string(), test_pdf_path(), HashMap::default())
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
            "What score must general education students achieve who first entered ninth grade in 1997 ?",
            sources,
            None,
            Effort::Medium,
        )
        .await
        .expect("could not call ask");

    let mut message_count = 0;
    let mut final_answer_received = false;

    while let Some(result) = stream.next().await {
        let message = result.expect("could not receive message from stream");
        message_count += 1;

        if let Some(ask_response_message::Message::FinalAnswer(final_answer)) = message.message {
            final_answer_received = true;

            let found_55 = final_answer
                .facts
                .iter()
                .any(|fact| fact.fact.contains("55"));

            assert!(
                found_55,
                "At least one fact in the final answer should contain '55', but facts were: {:?}",
                final_answer
                    .facts
                    .iter()
                    .map(|f| &f.fact)
                    .collect::<Vec<_>>()
            );

            println!(
                "Final answer received with {} facts",
                final_answer.facts.len()
            );
            break;
        }
    }

    assert!(
        final_answer_received,
        "Should receive a final answer from ask stream (received {} messages)",
        message_count
    );
}
