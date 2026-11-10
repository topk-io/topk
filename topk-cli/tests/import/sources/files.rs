use futures::StreamExt;
use serde_json::json;
use test_context::test_context;
use test_macros::rstest_ctx;
use topk::import::Target;

use crate::import::common::*;

#[test_context(Scratch)]
#[tokio::test]
async fn glob_resume_preserves_order(ctx: &mut Scratch) {
    for (name, start) in [("a", 100), ("b", 0), ("c", 200)] {
        ctx.seed_parquet(name, rows(start, 3)).await;
    }
    let target = Target {
        from: format!("{}/*.parquet", ctx.scratch().display()),
        ..Default::default()
    };
    let source = topk::import::Source::connect(
        &target.from.parse::<topk::import::Uri>().unwrap(),
        &config(),
    )
    .await
    .unwrap();
    let mut scan = source.scan(&target, None).unwrap().chunks;
    let first = scan.next().await.unwrap().unwrap();
    assert_eq!(first.rows.len(), 3);
    let cursor = first.cursor.unwrap();
    assert_eq!(
        cursor.clone().offset().unwrap(),
        (ctx.scratch().join("a.parquet").display().to_string(), 3)
    );
    drop(scan);

    let mut resumed = source.scan(&target, Some(cursor)).unwrap().chunks;
    let mut ids = Vec::new();
    while let Some(chunk) = resumed.next().await {
        for row in chunk.unwrap().rows {
            let row = row.unwrap();
            ids.push(
                row.iter()
                    .find(|(name, _)| name == "_id")
                    .unwrap()
                    .1
                    .as_string()
                    .unwrap()
                    .to_string(),
            );
        }
    }
    assert_eq!(ids, ["0", "1", "2", "200", "201", "202"]);
}

#[rstest_ctx(Scratch)]
#[case::ordered_error(None)]
#[case::limit_stops_before_error(Some(10_000))]
async fn glob_error_follows_prior_rows(ctx: &mut Scratch, #[case] limit: Option<u64>) {
    ctx.seed_parquet("a", rows(0, 10_000)).await;
    std::fs::write(ctx.scratch().join("b.parquet"), "broken source").unwrap();
    let target = Target {
        from: format!("{}/*.parquet", ctx.scratch().display()),
        limit,
        ..Default::default()
    };
    let source = topk::import::Source::connect(
        &target.from.parse::<topk::import::Uri>().unwrap(),
        &config(),
    )
    .await
    .unwrap();
    let mut scan = source.scan(&target, None).unwrap().chunks;
    let mut count = 0;
    let mut chunks = 0;
    let mut error = None;
    while let Some(chunk) = scan.next().await {
        match chunk {
            Ok(chunk) => {
                count += chunk
                    .rows
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap()
                    .len();
                chunks += 1;
            }
            Err(e) => {
                error = Some(e.to_string());
                break;
            }
        }
    }
    assert_eq!(count, 10_000);
    assert!(chunks > 2, "must exceed the reader channel capacity");
    match limit {
        Some(_) => assert!(error.is_none(), "{error:?}"),
        None => assert!(error.unwrap().contains("b.parquet")),
    }
}

#[rstest_ctx(Scratch)]
#[case::decimal_list(
    "SELECT 1 AS id, [1.50, 2.25, 3.00]::DECIMAL(10,2)[] AS prices",
    "prices = { type = 'float_list' }", json!({"_id": "1", "prices": [1.5, 2.25, 3.0]})
)]
#[case::bytes_as_text(
    "SELECT 1 AS id, 'hello'::BLOB AS a, '42'::BLOB AS b",
    "a = { type = 'text' }\nb = { type = 'text' }", json!({"_id": "1", "a": "hello", "b": "42"})
)]
#[case::empty_lists(
    "SELECT 1 AS id, []::VARCHAR[] AS names, []::INTEGER[] AS numbers",
    "names = { type = 'text_list' }\nnumbers = { type = 'int_list' }", json!({"_id": "1", "names": [], "numbers": []})
)]
async fn decodes(
    ctx: &mut Scratch,
    #[case] select: &str,
    #[case] fields: &str,
    #[case] expected: serde_json::Value,
) {
    let from = ctx.parquet(select);
    assert_eq!(
        stream(None, &target(&from, "id", fields)).await.unwrap(),
        [(None, expected)]
    );
}

#[test_context(Scratch)]
#[tokio::test]
async fn ragged_csv_keeps_its_header(ctx: &mut Scratch) {
    let from = ctx.csv(
        "ragged",
        "id,name,price\n1,apple,1.20\n2,banana\n3,cherry,2.40,extra",
    );
    let docs = stream(None, &target(&from, "id", "name = { type = 'text' }"))
        .await
        .unwrap();
    assert_eq!(docs.len(), 3);
    assert_eq!(docs[1].1, json!({"_id": "2", "name": "banana"}));
}

#[test_context(Scratch)]
#[tokio::test]
async fn empty_unsupported_list_is_rejected(ctx: &mut Scratch) {
    let from = ctx.parquet("SELECT 1 AS id, []::BOOLEAN[] AS flags");
    let error = stream(
        None,
        &target(&from, "id", "flags = { type = 'float_list' }"),
    )
    .await
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("list of Boolean has no equivalent"),
        "{error}"
    );
}

#[test_context(Scratch)]
#[tokio::test]
async fn bad_filter_names_the_filter(ctx: &mut Scratch) {
    let from = ctx.seed_parquet("books", books()).await;
    let mut target = target(&from, "_id", "title = { type = 'text' }");
    target.filter = Some("bogus(".into());
    let error = stream(None, &target).await.unwrap_err().to_string();
    assert!(error.contains("bogus("), "{error}");
    assert!(!error.contains("SELECT"), "{error}");
}
