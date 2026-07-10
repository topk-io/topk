mod common;

use elasticsearch::cluster::ClusterHealthParts;

#[tokio::test]
async fn test_root_returns_elastic_product_header() {
    let client = common::Client::new();
    let res = client.es().info().send().await.expect("info");

    assert!(res.status_code().is_success());
    assert_eq!(
        res.headers()
            .get("x-elastic-product")
            .and_then(|v| v.to_str().ok()),
        Some("Elasticsearch"),
        "missing x-elastic-product header"
    );

    let body: serde_json::Value = res.json().await.unwrap();
    assert!(body["version"]["number"].is_string());
}

#[tokio::test]
async fn test_cluster_health() {
    let client = common::Client::new();
    let res = client
        .es()
        .cluster()
        .health(ClusterHealthParts::None)
        .send()
        .await
        .expect("cluster health");
    assert!(res.status_code().is_success());

    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["status"], "green", "{body}");
}

#[tokio::test]
async fn test_license() {
    let client = common::Client::new();
    let res = client
        .es()
        .license()
        .get()
        .send()
        .await
        .expect("license get");
    assert!(res.status_code().is_success());

    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["license"]["status"], "active", "{body}");
}
