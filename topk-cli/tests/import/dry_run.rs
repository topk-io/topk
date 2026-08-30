use crate::common::*;
use serde_json::json;
use test_context::test_context;
use topk::import::{Field, Spec, Target, Type};

#[test_context(Scratch)]
#[tokio::test]
async fn limit(ctx: &mut Scratch) {
    let object = ctx.seed_parquet("big", rows(1, 100)).await;
    let target = Target {
        limit: Some(5),
        ..object
    };
    assert_eq!(stream_docs(&target).await.unwrap().len(), 5);
}

#[test_context(Scratch)]
#[tokio::test]
async fn glob(ctx: &mut Scratch) {
    for (i, base) in [1_u64, 11, 21].iter().enumerate() {
        ctx.seed_parquet(&format!("part_{i}"), rows(*base, 3)).await;
    }
    let target = Target {
        from: format!("{}/part_*.parquet", ctx.scratch().display()),
        ..Default::default()
    };
    assert_eq!(
        stream_docs(&target).await.unwrap().len(),
        9,
        "all 3 files × 3 rows"
    );
}

#[test_context(Scratch)]
#[tokio::test]
async fn deterministic_output(ctx: &mut Scratch) {
    let object = ctx.seed_parquet("books", books()).await;
    let spec = ctx.target_spec("c", object);

    let runs: Vec<String> = (0..3)
        .map(|_| ok(&["import", "-f", &spec, "--dry-run"], &[]))
        .collect();
    assert_eq!(runs[0], runs[1], "printed spec differs between runs");
    assert_eq!(runs[1], runs[2], "printed spec differs between runs");
    assert_eq!(
        toml::from_str::<Spec>(&runs[0])
            .expect("the printed spec re-parses")
            .collections
            .len(),
        1
    );

    let docs = dry_run(&spec, &[]);
    assert!(!docs.is_empty(), "documents preview on stderr");
}

#[test_context(Scratch)]
#[tokio::test]
async fn preview_is_capped(ctx: &mut Scratch) {
    let object = ctx.seed_parquet("big", rows(1, 20)).await;
    let spec = ctx.target_spec("c", object);
    assert_eq!(dry_run(&spec, &[]).len(), 5);
}

#[test_context(Scratch)]
#[tokio::test]
async fn decimal_list(ctx: &mut Scratch) {
    let target = Target {
        from: ctx.sql_parquet(
            "decimals",
            "SELECT 1 AS id, [1.50, 2.25, 3.00]::DECIMAL(10,2)[] AS prices",
        ),
        id: Some("id".to_string()),
        fields: fields([(
            "prices",
            Field {
                ty: Type::FloatList,
                ..Default::default()
            },
        )]),
        ..Default::default()
    };
    let docs = stream_docs(&target).await.unwrap();
    assert_eq!(docs["1"], json!({"_id": "1", "prices": [1.5, 2.25, 3.0]}));
}

#[test_context(Scratch)]
#[tokio::test]
async fn bytes_as_text(ctx: &mut Scratch) {
    let target = Target {
        from: ctx.sql_parquet(
            "bytes",
            "SELECT 1 AS id, 'hello'::BLOB AS a, '42'::BLOB AS b",
        ),
        id: Some("id".to_string()),
        fields: fields([
            (
                "a",
                Field {
                    ty: Type::Text,
                    ..Default::default()
                },
            ),
            (
                "b",
                Field {
                    ty: Type::Text,
                    ..Default::default()
                },
            ),
        ]),
        ..Default::default()
    };
    let docs = stream_docs(&target).await.unwrap();
    assert_eq!(docs["1"], json!({"_id": "1", "a": "hello", "b": "42"}));
}

#[test_context(Scratch)]
#[tokio::test]
async fn missing_id(ctx: &mut Scratch) {
    let file = ctx.scratch().join("ragged.csv");
    std::fs::write(&file, "id,a,b\n1,x,y\n2,z\n").unwrap();
    let target = Target {
        from: file.display().to_string(),
        id: Some("id".to_string()),
        ..Default::default()
    };
    let err = stream_docs(&target)
        .await
        .expect_err("ragged row must fail");
    assert!(err.to_string().contains("which has:"), "got: {err}");
}

#[test_context(Scratch)]
#[tokio::test]
async fn id_placeholder(ctx: &mut Scratch) {
    let file = ctx.scratch().join("rows.csv");
    std::fs::write(&file, "a,b\nx,y\n").unwrap();
    let spec = ctx.target_spec(
        "c",
        Target {
            from: file.display().to_string(),
            id: Some("<column>".to_string()),
            fields: fields([(
                "a",
                Field {
                    ty: Type::Text,
                    ..Default::default()
                },
            )]),
            ..Default::default()
        },
    );
    // A dry run renders the template, placeholder included, so it can be
    // captured and filled in; only a real import insists on a resolved id.
    let out = ok(&["import", "-f", &spec, "--dry-run"], &[]);
    assert!(
        out.contains(r#"id = "<column>""#),
        "dry-run must render the placeholder spec:\n{out}"
    );
    let err = fails(&["import", "-f", &spec, "--yes"], &[]);
    assert!(
        err.contains("--id"),
        "a real import must point at --id:\n{err}"
    );
}

#[test_context(Scratch)]
#[tokio::test]
async fn bad_filter_names_the_filter(ctx: &mut Scratch) {
    let object = ctx.seed_parquet("books", books()).await;
    let target = Target {
        filter: Some("bogus(".to_string()),
        ..object
    };

    let message = stream_docs(&target).await.unwrap_err().to_string();
    assert!(message.contains("bogus("), "got: {message}");
    assert!(!message.contains("SELECT"), "got: {message}");
}

#[test_context(Scratch)]
#[tokio::test]
async fn missing_spec_names_the_path(_ctx: &mut Scratch) {
    let err = fails(&["import", "-f", "/nope/spec.toml"], &[]);
    assert!(err.contains("/nope/spec.toml"), "got: {err}");
}
