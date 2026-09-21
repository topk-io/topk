use test_context::test_context;

use super::common::CliTestContext;

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
