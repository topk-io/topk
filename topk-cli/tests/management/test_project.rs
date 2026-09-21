use serde_json::json;
use test_context::test_context;

use super::common::{unique_name, CliTestContext};

#[test_context(CliTestContext)]
#[tokio::test]
async fn create_get_list_delete(ctx: &mut CliTestContext) {
    let name = unique_name("project");
    let created = ctx.create_project(&name);
    assert_eq!(created["name"], name);
    let id = created["project_id"].as_str().unwrap().to_string();
    assert!(!id.is_empty());
    assert!(!created["org_id"].as_str().unwrap().is_empty());
    assert!(created["created_at"].as_i64().unwrap() > 0);

    assert_eq!(
        ctx.json(&["project", "get", &id]),
        std::slice::from_ref(&created)
    );
    assert!(ctx.json(&["project", "list"]).contains(&created));

    let text = ctx.ok(&["project", "get", &id]);
    assert!(text.contains("Project ID:") && text.contains(&id), "{text}");
    let text = ctx.ok(&["project", "list"]);
    assert!(
        text.contains("Project ID") && text.contains(&name),
        "{text}"
    );

    // Without a terminal there is nobody to confirm.
    assert!(ctx
        .fails(&["project", "delete", &id])
        .contains("pass --yes"));
    assert!(ctx.json(&["project", "get", &id]).len() == 1);

    assert_eq!(
        ctx.json(&["project", "delete", "--yes", &id]),
        [json!({"deleted": true})]
    );
    assert!(ctx.fails(&["project", "get", &id]).contains("not found"));
    assert!(ctx
        .fails(&["project", "delete", "-y", &id])
        .contains("not found"));
    assert!(!ctx.json(&["project", "list"]).contains(&created));
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn delete_prints_nothing_in_text_mode(ctx: &mut CliTestContext) {
    let id = ctx.create_project(&unique_name("project"))["project_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(ctx.ok(&["project", "delete", "-y", &id]), "");
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn invalid_project_id_is_rejected(ctx: &mut CliTestContext) {
    let stderr = ctx.fails(&["project", "get", "nope"]);
    assert!(stderr.contains("invalid project_id"), "{stderr}");
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn commands_need_a_session(ctx: &mut CliTestContext) {
    ctx.ok(&["logout"]);
    for args in [["project", "list"], ["region", "list"]] {
        let stderr = ctx.fails(&args);
        assert!(stderr.contains("not logged in"), "{stderr}");
        assert!(stderr.contains("topk login"), "{stderr}");
    }
}

#[tokio::test]
async fn login_is_required_before_any_command() {
    let ctx = CliTestContext::logged_out();
    let stderr = ctx.fails(&["project", "list"]);
    assert!(stderr.contains("not logged in"), "{stderr}");
}
