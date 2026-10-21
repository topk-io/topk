use rstest::rstest;

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
#[case::default("SET consistency_level = 'default'")]
#[case::indexed("SET consistency_level = 'indexed'")]
#[case::strong("SET consistency_level = 'strong'")]
#[case::unknown_variable("SET unknown_var = 'x'")]
#[case::invalid_consistency("SET consistency_level = 'eventual'")]
#[case::show("SHOW consistency_level")]
fn session_variables_are_unsupported(#[case] sql: &str) {
    let error = topk_sql::convert_sql(topk_sql::parse_sql(sql).unwrap()).unwrap_err();
    assert!(matches!(error, topk_sql::Error::Unsupported(_)), "{error}");
}
