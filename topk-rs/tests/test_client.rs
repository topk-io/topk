use test_context::test_context;
use topk_rs::query::{field, filter, r#match};
use topk_rs::{Client, ClientConfig, Error};

mod utils;
use utils::ProjectTestContext;

#[tokio::test]
async fn test_invalid_api_key() {
    let host = std::env::var("TOPK_HOST").unwrap_or("topk.io".to_string());
    let region = std::env::var("TOPK_REGION").unwrap_or("elastica".to_string());
    let https = std::env::var("TOPK_HTTPS").unwrap_or("true".to_string()) == "true";

    let client = Client::new(
        ClientConfig::new("INVALID_API_KEY", region)
            .with_host(host)
            .with_https(https),
    );

    let response = client
        .collections()
        .list()
        .await
        .expect_err("should not be able to list collections");

    assert!(matches!(response, Error::PermissionDenied));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_protobuf_recursion_limit_returns_invalid_argument(ctx: &mut ProjectTestContext) {
    // Build a deeply nested OR expression that exceeds protobuf recursion limit (100)
    let mut deep_expr = r#match("test", Some("field"), None, false);
    for i in 0..200 {
        deep_expr = deep_expr.or(r#match(&format!("term{}", i), Some("field"), None, false));
    }

    let err = ctx
        .client
        .collection("test")
        .query(filter(deep_expr).topk(field("id"), 10, true), None, None)
        .await
        .expect_err("Query should fail due to protobuf recursion limit");

    assert!(
        matches!(err, Error::InvalidArgument(_) if err.to_string().contains("failed to decode Protobuf message")),
        "Expected InvalidArgument error with 'failed to decode Protobuf message', but got: {}",
        err
    );
}
