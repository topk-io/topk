use test_context::test_context;

use crate::common::session::{unique_name, CliTestContext};

#[test_context(CliTestContext)]
#[tokio::test]
async fn usage_of_the_organization(ctx: &mut CliTestContext) {
    let rows = ctx.json(&["usage", "--since", "1h"]);
    assert!(!rows.is_empty());
    for row in &rows {
        assert!(row.get("project_id").is_none(), "{row}");
        assert!(row.get("collection").is_none(), "{row}");
        assert!(row["value"].is_u64(), "{row}");
        assert!(
            ["count", "bytes", "byte_hours"].contains(&row["unit"].as_str().unwrap()),
            "{row}"
        );
    }

    let text = ctx.ok(&["usage", "--since", "1h"]);
    assert!(text.contains("Metric") && text.contains("Value"), "{text}");
    assert!(!text.contains("Project"), "{text}");
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn usage_of_a_project(ctx: &mut CliTestContext) {
    let id = ctx.create_project(&unique_name("usage"))["project_id"]
        .as_str()
        .unwrap()
        .to_string();

    let rows = ctx.json(&[
        "usage",
        "--since",
        "2026-01-01",
        "--until",
        "2026-01-02T00:00:00Z",
        "--project-id",
        &id,
    ]);
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|row| row["project_id"] == id.as_str()));
    // A project created now has no usage in the past.
    assert!(rows.iter().all(|row| row["value"] == 0), "{rows:?}");

    let text = ctx.ok(&["usage", "--since", "1d", "--project-id", &id]);
    assert!(text.contains("Project") && text.contains(&id), "{text}");
    assert!(!text.contains("Collection"), "{text}");
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn usage_defaults_to_the_selected_project(ctx: &mut CliTestContext) {
    let id = ctx.select_e2e_project();
    let rows = ctx.json(&["usage", "--since", "1h"]);
    assert!(rows.iter().all(|row| row["project_id"] == id.as_str()));
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn usage_rejects_invalid_arguments(ctx: &mut CliTestContext) {
    assert!(ctx
        .fails(&["usage"])
        .contains("required arguments were not provided"));
    assert!(ctx
        .fails(&["usage", "--since", "42x"])
        .contains("expected a duration"));
    assert!(ctx
        .fails(&["usage", "--since", "1h", "--collection", "books"])
        .contains("--collection needs a project"));

    let id = ctx.create_project(&unique_name("usage"))["project_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(ctx
        .fails(&[
            "usage",
            "--since",
            "1h",
            "--project-id",
            &id,
            "--collection",
            "missing"
        ])
        .contains("not found"));
}
