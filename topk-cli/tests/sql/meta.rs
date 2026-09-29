use rstest::rstest;

use topk::commands::sql::meta::expand;

#[test]
fn dg_is_du() {
    assert_eq!(expand(r"\dg").unwrap(), expand(r"\du").unwrap());
}

#[test]
fn describe_without_a_table_lists_tables() {
    assert_eq!(expand(r"\d").unwrap(), expand(r"\dt").unwrap());
    let meta = expand(r"\d books").unwrap().unwrap();
    assert_eq!(meta.describes.as_deref(), Some("books"));
    let sql = meta.sql;
    assert!(sql.contains("FROM information_schema.columns"));
    assert!(sql.contains("table_name = 'books'"));
}

#[rstest]
#[case(r"\? books", r"\? takes no arguments")]
#[case(r"\? books authors", r"\? takes no arguments")]
#[case(r"\dt books authors", r"\dt takes at most one argument")]
#[case(r"\x", r"unknown command \x; run \? for the supported commands")]
fn rejects_invalid_commands(#[case] input: &str, #[case] message: &str) {
    assert_eq!(expand(input).unwrap_err().to_string(), message);
}

#[rstest]
#[case("books*?", "books%_")]
#[case(r"a_b%c\d", r"a\_b\%c\\d")]
#[case("café*", "café%")]
fn converts_patterns(#[case] input: &str, #[case] expected: &str) {
    for (command, column) in [
        ("dt", "table_name"),
        ("di", "indexname"),
        ("dA", "amname"),
        ("dT", "typname"),
        ("dn", "schema_name"),
        ("l", "datname"),
        ("du", "rolname"),
    ] {
        let sql = expand(&format!("\\{command} {input}"))
            .unwrap()
            .unwrap()
            .sql;
        assert!(
            sql.contains(&format!("{column} LIKE '{expected}' ESCAPE '\\'")),
            "{sql}"
        );
    }
}
