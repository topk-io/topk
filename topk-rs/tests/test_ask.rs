use futures_util::StreamExt;
use test_context::test_context;

use topk_rs::proto::v1::{
    ctx::{ask_result::Message, AskResult},
    data::Value,
};
use topk_rs::{Client, ClientConfig, Error};

mod utils;
use utils::ProjectTestContext;

use crate::utils::dataset::test_pdf;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_ask(ctx: &mut ProjectTestContext) {
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

    // Wait for file to be processed
    ctx.client
        .dataset(&dataset.name)
        .wait_for_handle(&handle, None)
        .await
        .expect("could not wait handle");

    // Ask
    let stream = ctx
        .client
        .ask(
            "What is bootstraping in programming language design?",
            [&dataset.name],
            None,
            None,
            None,
            Some(true),
        )
        .await
        .expect("could not call ask");

    let answer = collect_answer(stream).await;
    assert!(!answer.facts.is_empty(), "expected at least one fact");
    assert!(!answer.refs.is_empty(), "expected at least one ref");
    assert!(
        answer.refs.iter().all(|(_, sr)| sr.content.is_some()),
        "expected all refs to have content"
    );
}

#[tokio::test]
async fn test_ask_empty_datasets() {
    let err = Client::new(ClientConfig::new("dummy-key", "us-east-1"))
        .ask("query", Vec::<&str>::new(), None, None, None, Some(true))
        .await
        .expect_err("should fail with empty datasets");

    assert!(
        matches!(err, Error::InvalidArgument(ref s) if s == "provide at least one dataset"),
        "unexpected error: {err}"
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_ask_include_content_true_returns_chunks(ctx: &mut ProjectTestContext) {
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
        .expect("could not wait handle");

    let stream = ctx
        .client
        .ask(
            "What is bootstraping in programming language design?",
            [&dataset.name],
            None,
            None,
            None,
            Some(true),
        )
        .await
        .expect("could not call ask");

    let answer = collect_answer(stream).await;
    assert!(!answer.facts.is_empty(), "expected at least one fact");
    assert!(!answer.refs.is_empty(), "expected at least one ref");
    assert!(
        answer.refs.iter().all(|(_, sr)| sr.content.is_some()),
        "expected all refs to have content"
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_ask_include_content_false_strips_chunks(ctx: &mut ProjectTestContext) {
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
        .expect("could not wait handle");

    let stream = ctx
        .client
        .ask(
            "What is bootstraping in programming language design?",
            [&dataset.name],
            None,
            None,
            None,
            Some(false),
        )
        .await
        .expect("could not call ask");

    let answer = collect_answer(stream).await;
    assert!(!answer.facts.is_empty(), "expected at least one fact");
    assert!(!answer.refs.is_empty(), "expected at least one ref");
    assert!(
        answer.refs.iter().all(|(_, sr)| sr.content.is_none()),
        "expected all refs to have no content"
    );
}

async fn collect_answer(
    mut stream: tonic::Streaming<AskResult>,
) -> topk_rs::proto::v1::ctx::ask_result::Answer {
    while let Some(msg) = stream.next().await {
        if let Some(Message::Answer(answer)) = msg.unwrap().message {
            return answer;
        }
    }
    unreachable!("no Answer message received");
}
