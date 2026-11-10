use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::http::{StatusCode, Uri};
use axum::Router;
use elasticsearch::indices::IndicesStatsParts;
use futures::StreamExt;
use rstest::rstest;
use serde_json::json;
use test_macros::rstest_ctx;
use topk::import::Target;
use topk_rs::doc;

use crate::import::common::seed::es::Es;
use crate::import::common::seed::Backend::Elasticsearch;
use crate::import::common::*;

/// A query elasticsearch rejects comes back with elasticsearch's reason, not
/// just the status line — `error_for_status_code` throws that body away.
#[tokio::test]
async fn es_rejected_query_carries_the_reason() {
    let source = Elasticsearch.seed(books()).await;
    let mut target = target(&source.from, "_id", "title = { type = 'text' }");
    target.filter = Some(r#"{"bogus_query": {}}"#.into());
    let error = stream(source.url.as_deref(), &target)
        .await
        .unwrap_err()
        .to_string();

    // The status line does not name the query; only the body does.
    assert!(error.contains("400"), "{error}");
    assert!(error.contains("bogus_query"), "{error}");
}

#[tokio::test]
async fn elasticsearch_url_index_is_a_selector() {
    let source = Elasticsearch.seed(books()).await;
    let target = target(&source.from, "_id", "title = { type = 'text' }");
    let url = format!("es://localhost:19200/{}", target.from);

    let catalog =
        topk::import::Source::connect(&url.parse::<topk::import::Uri>().unwrap(), &config())
            .await
            .unwrap()
            .catalog()
            .await
            .unwrap();
    let docs = stream(Some(&url), &target).await.unwrap();

    assert_eq!(catalog.len(), 1);
    assert_eq!(catalog[0].from, target.from);
    assert_eq!(docs.len(), 3);
}

#[tokio::test]
async fn expired_es_cursor_does_not_restart() {
    let opens = Arc::new(AtomicUsize::new(0));
    let seen = opens.clone();
    let app = Router::new().fallback(move |uri: Uri| {
        let seen = seen.clone();
        async move {
            if uri.path() == "/books/_pit" {
                seen.fetch_add(1, Ordering::Relaxed);
            }
            (
                StatusCode::NOT_FOUND,
                [("content-type", "application/json")],
                r#"{"error":{"reason":"expired cursor"}}"#,
            )
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let uri = format!("es://{addr}").parse::<topk::import::Uri>().unwrap();
    let source = topk::import::Source::connect(&uri, &config())
        .await
        .unwrap();
    let target = Target {
        limit: Some(2),
        ..target("books", "_id", "title = { type = 'text' }")
    };

    let scan = source
        .scan(
            &target,
            Some(topk::import::Cursor::Page {
                pit: "expired".to_string(),
                sort: vec![1],
            }),
        )
        .unwrap();
    let mut scan = scan.chunks;
    let error = scan.next().await.unwrap().err().unwrap().to_string();
    server.abort();

    assert!(error.contains("saved cursor unavailable"), "{error}");
    assert!(error.contains("start a new import"), "{error}");
    assert_eq!(opens.load(Ordering::Relaxed), 0, "must not reopen the PIT");
}

#[rstest]
#[case::array_filter("[]")]
#[case::null_filter("null")]
#[case::scalar_filter("42")]
#[case::malformed_filter("{")]
#[tokio::test]
async fn es_filter_requires_object(#[case] filter: &str) {
    let uri = "es+http://127.0.0.1:1"
        .parse::<topk::import::Uri>()
        .unwrap();
    let source = topk::import::Source::connect(&uri, &config())
        .await
        .unwrap();
    let target = Target {
        filter: Some(filter.to_string()),
        ..target("books", "_id", "title = { type = 'text' }")
    };

    let error = refused(source.scan(&target, None));

    assert!(error.contains("is not a JSON object"), "{error}");
}

#[rstest_ctx(Scratch)]
#[case::discovered_catalog(false, None)]
#[case::supplied_spec(true, None)]
#[case::missing_id(false, Some("--id"))]
#[case::missing_partition(false, Some("--partition"))]
async fn validation_reads_metadata_once(
    ctx: &mut Scratch,
    #[case] supplied: bool,
    #[case] missing: Option<&str>,
) {
    let mappings = Arc::new(Mutex::new(Vec::new()));
    let seen = mappings.clone();
    let app = Router::new().fallback(move |uri: Uri| {
        let seen = seen.clone();
        async move {
            let body = match uri.path() {
                "/_mapping" | "/books/_mapping" => {
                    seen.lock().unwrap().push(uri.path().to_string());
                    json!({"books": {"mappings": {"properties": {"title": {"type": "text"}}}}})
                }
                "/books/_pit" => json!({"id": "test-pit"}),
                "/_search" => json!({"hits": {"hits": []}}),
                "/_pit" => json!({"succeeded": true, "num_freed": 1}),
                _ => {
                    return (
                        StatusCode::NOT_FOUND,
                        [("content-type", "application/json")],
                        "{}".into(),
                    )
                }
            };
            (
                StatusCode::OK,
                [("content-type", "application/json")],
                body.to_string(),
            )
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let url = format!("es://{addr}{}", if supplied { "/unrelated" } else { "" });
    let mut command = topk().args(["import", &url, "--dry-run"]);
    if supplied {
        let spec = ctx.multi_spec([(
            "books".to_string(),
            target("books", "_id", "title = { type = 'text' }"),
        )]);
        command = command.args(["-f", &spec]);
    }
    if let Some(flag) = missing {
        command = command.args([flag, "absent"]);
    }

    let output = tokio::task::spawn_blocking(move || command.output())
        .await
        .unwrap();
    server.abort();

    let error = String::from_utf8(output.stderr).unwrap();
    assert_eq!(output.status.success(), missing.is_none(), "{error}");
    if missing.is_some() {
        assert!(error.contains("column \"absent\" is not in"), "{error}");
    }
    assert_eq!(
        *mappings.lock().unwrap(),
        [if supplied {
            "/books/_mapping"
        } else {
            "/_mapping"
        }],
    );
}

/// Nothing saved refers to the point in time of a scan that failed or was
/// abandoned, so it must be closed rather than held for `KEEP_ALIVE`.
#[rstest]
#[case::clean_scan("{\"match_all\": {}}", "title = { type = 'text' }")]
#[case::rejected_query(r#"{"bogus_query": {}}"#, "title = { type = 'text' }")]
#[case::bad_row("{\"match_all\": {}}", "absent = { type = 'text', required = true }")]
#[tokio::test]
async fn failed_es_scan_closes_its_pit(#[case] filter: &str, #[case] fields: &str) {
    let source = Elasticsearch.seed(books()).await;
    let mut target = target(&source.from, "_id", fields);
    target.filter = Some(filter.into());

    let result = stream(source.url.as_deref(), &target).await;
    if filter.contains("bogus_query") {
        assert!(refused(result).contains("bogus_query"));
    } else if fields.contains("required") {
        assert!(refused(result).contains("required field is missing"));
    } else {
        assert_eq!(result.unwrap().len(), 3);
    }

    let stats: serde_json::Value = Es::client()
        .indices()
        .stats(IndicesStatsParts::IndexMetric(&[&source.from], &["search"]))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        stats["_all"]["total"]["search"]["open_contexts"], 0,
        "{stats}"
    );
}
