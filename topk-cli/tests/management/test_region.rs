use test_context::test_context;

use crate::common::session::CliTestContext;

#[test_context(CliTestContext)]
#[tokio::test]
async fn list_regions(ctx: &mut CliTestContext) {
    let regions = ctx.json(&["region", "list"]);
    assert!(!regions.is_empty());
    let names: Vec<&str> = regions
        .iter()
        .map(|region| region["name"].as_str().expect("region name"))
        .collect();
    assert!(names.iter().all(|name| !name.is_empty()));

    let text = ctx.ok(&["region", "list"]);
    assert!(text.contains("Name"), "{text}");
    assert!(names.iter().all(|name| text.contains(name)), "{text}");
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn select_current_clear(ctx: &mut CliTestContext) {
    let regions = ctx.json(&["region", "list"]);
    let name = regions[0]["name"].as_str().expect("region name");
    assert_eq!(regions[0]["selected"], false);

    ctx.ok(&["region", "select", name]);
    assert_eq!(
        ctx.ok(&["region", "current"]).trim(),
        format!("Default region: {name}")
    );
    let selected: Vec<_> = ctx
        .json(&["region", "list"])
        .into_iter()
        .filter(|region| region["selected"] == true)
        .collect();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0]["name"], name);
    assert!(ctx.ok(&["region", "list"]).contains(&format!("{name}*")));

    ctx.ok(&["region", "clear"]);
    assert!(ctx
        .fails(&["region", "current"])
        .contains("no region selected"));
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn select_rejects_unknown_or_missing_region(ctx: &mut CliTestContext) {
    assert!(ctx
        .fails(&["region", "select", "nowhere"])
        .contains("unknown region \"nowhere\""));
    assert!(ctx
        .fails(&["region", "select"])
        .contains("stdin is not a terminal"));
}
