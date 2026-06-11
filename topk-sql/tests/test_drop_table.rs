use rstest::rstest;

mod common;
use common::{Scope, TableScope};

#[rstest]
#[case::minimal(
    "CREATE TABLE {{table}} (name TEXT NOT NULL, score FLOAT)",
    "DROP TABLE {{table}}"
)]
#[tokio::test]
async fn drop_table(#[case] create_sql: &str, #[case] drop_sql: &str) {
    TableScope::with_scope(async |ctx| -> anyhow::Result<()> {
        ctx.sql(create_sql).await?;
        ctx.sql(drop_sql).await?;

        match ctx.sql("SELECT name FROM {{table}}").await {
            Ok(_) => anyhow::bail!("table still exists"),
            Err(err) => {
                if err.to_string() == "Table does not exist" {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("unexpected error: {err}"))
                }
            }
        }
    })
    .await
    .unwrap();
}

#[rstest]
#[case::missing(None, "DROP TABLE IF EXISTS {{table}}")]
#[case::after_drop(
    Some("CREATE TABLE {{table}} (name TEXT NOT NULL)"),
    "DROP TABLE IF EXISTS {{table}}"
)]
#[tokio::test]
async fn drop_table_if_exists(#[case] setup_sql: Option<&str>, #[case] drop_sql: &str) {
    TableScope::with_scope(async |ctx| {
        if let Some(setup) = setup_sql {
            ctx.sql(setup).await?;
            ctx.sql("DROP TABLE {{table}}").await?;
        }
        ctx.sql(drop_sql).await
    })
    .await
    .unwrap();
}

#[rstest]
#[case::missing_table(None, "DROP TABLE {{table}}", "Table does not exist")]
#[case::after_drop(
    Some("CREATE TABLE {{table}} (name TEXT NOT NULL)"),
    "DROP TABLE {{table}}",
    "Table does not exist"
)]
#[tokio::test]
async fn drop_table_rejected(
    #[case] setup_sql: Option<&str>,
    #[case] query: &str,
    #[case] expected: &str,
) {
    let err = TableScope::with_scope(async |ctx| -> anyhow::Result<()> {
        if let Some(setup) = setup_sql {
            ctx.sql(setup).await?;
            ctx.sql("DROP TABLE {{table}}").await?;
        }
        ctx.sql(query).await?;
        Ok(())
    })
    .await
    .unwrap_err();

    assert_eq!(err.to_string(), expected);
}
