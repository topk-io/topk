use rstest::rstest;
use topk_rs::doc;

mod common;
use common::{Scope, SessionContext};

#[rstest]
#[case::begin("BEGIN")]
#[case::commit("COMMIT")]
#[case::rollback("ROLLBACK")]
#[case::discard("DISCARD ALL")]
#[tokio::test]
async fn noop_transaction(#[case] sql: &str) {
    SessionContext::with_scope(async |client| client.sql(sql).await)
        .await
        .unwrap();
}

#[rstest]
#[case::default("SET consistency_level = 'default'", "default")]
#[case::indexed("SET consistency_level = 'indexed'", "indexed")]
#[case::strong("SET consistency_level = 'strong'", "strong")]
#[tokio::test]
async fn consistency_level(#[case] sql: &str, #[case] expected: &str) {
    let rows = SessionContext::with_scope(async |client| {
        client.batch(&[sql, "SHOW consistency_level"]).await
    })
    .await
    .unwrap();
    assert_eq!(rows, vec![doc!("consistency_level" => expected)]);
}

#[rstest]
#[case::unknown_variable("SET unknown_var = 'x'", "Invalid: unknown variable: unknown_var")]
#[case::invalid_consistency(
    "SET consistency_level = 'eventual'",
    "Invalid: SET consistency_level: must be one of 'indexed', 'strong', or 'default'"
)]
#[tokio::test]
async fn session_rejected(#[case] sql: &str, #[case] expected: &str) {
    let err = SessionContext::with_scope(async |client| client.sql(sql).await)
        .await
        .unwrap_err();
    assert_eq!(err.to_string(), expected);
}
