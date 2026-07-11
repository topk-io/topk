mod common;

use common::TestScope;
use elasticsearch::http::Method;
use test_context::test_context;

#[test_context(TestScope)]
#[tokio::test]
async fn test_options_search_returns_allow(scope: &TestScope) {
    scope.create().await;

    let res = scope
        .client
        .es()
        .transport()
        .send(
            Method::Options,
            &format!("/{}/_search", scope.name),
            Default::default(),
            None::<&()>,
            None::<&[u8]>,
            None,
        )
        .await
        .expect("options");

    assert_eq!(res.status_code(), 200);
    assert_eq!(
        res.headers().get("allow").and_then(|v| v.to_str().ok()),
        Some("GET,POST"),
        "missing Allow header"
    );
    assert_eq!(
        res.headers()
            .get("x-elastic-product")
            .and_then(|v| v.to_str().ok()),
        Some("Elasticsearch")
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_search_without_content_type_returns_406(scope: &TestScope) {
    scope.create().await;

    let body = br#"{"query":{"match_all":{}}}"#;
    let res = scope
        .client
        .es()
        .transport()
        .send(
            Method::Post,
            &format!("/{}/_search", scope.name),
            Default::default(),
            None::<&()>,
            Some(&body[..]),
            None,
        )
        .await
        .expect("search");

    assert_eq!(res.status_code(), 406, "missing Content-Type should be 406");
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["status"], 406, "{body}");
    assert_eq!(body["error"]["type"], "media_type_header_exception", "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_search_with_unsupported_content_type_returns_406(scope: &TestScope) {
    scope.create().await;

    let mut headers = elasticsearch::http::headers::HeaderMap::new();
    headers.insert(
        "Content-Type",
        http::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );

    let body = br#"{"query":{"match_all":{}}}"#;
    let res = scope
        .client
        .es()
        .transport()
        .send(
            Method::Post,
            &format!("/{}/_search", scope.name),
            headers,
            None::<&()>,
            Some(&body[..]),
            None,
        )
        .await
        .expect("search");

    assert_eq!(res.status_code(), 406, "unsupported Content-Type should be 406");
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["status"], 406, "{body}");
    assert_eq!(body["error"]["type"], "media_type_header_exception", "{body}");
}
