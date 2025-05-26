use topk_rs::{Client, ClientConfig, Error};

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
