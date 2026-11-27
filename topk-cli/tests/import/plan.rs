use crate::import::common::refused;
use rstest::rstest;
use topk::import::plan::{Field, FieldSelection, Plan, Target};
use topk::import::source::SourceLocation;
use topk::import::source::Table;
use topk_rs::proto::v1::control::field_type_list::ListValueType;
use topk_rs::proto::v1::control::{FieldIndex, FieldSpec, KeywordIndexType};

fn table(columns: &[&str]) -> Table {
    Table {
        from: SourceLocation::Object("books".into()),
        key: None,
        complete: true,
        columns: columns
            .iter()
            .map(|name| ((*name).into(), FieldSpec::text(false)))
            .collect(),
    }
}

#[test]
fn explicit_type_clears_source_index() {
    let selection: FieldSelection = "title=text".parse().unwrap();
    let mut field =
        Field::new(FieldSpec::text(true).with_index(FieldIndex::keyword(KeywordIndexType::Text)));
    selection.apply(&mut field, None).unwrap();
    assert_eq!(field.schema, FieldSpec::text(true));
}

/// Field names may not start with `_`, but sources use that for their own
/// bookkeeping columns — so a discovered plan renames them and runs as written.
#[test]
fn underscore_columns_are_renamed() {
    let metadata = table(&["_id", "_lang", "_n_chars", "text"]);
    let target = Target::new(&metadata, None).unwrap();

    assert_eq!(target.id, "_id");
    assert_eq!(target.fields["lang"].from.as_deref(), Some("_lang"));
    assert_eq!(target.fields["n_chars"].from.as_deref(), Some("_n_chars"));
    // A column that never needed renaming keeps its name and carries no `from`.
    assert_eq!(target.fields["text"].from.as_deref(), None);
    assert!(!target.fields.contains_key("_lang"));
}

/// A stripped name that collides is left alone, so validation rejects it: a
/// clear error beats silently folding two columns onto one field.
#[test]
fn underscore_rename_yields_to_a_collision() {
    let metadata = table(&["_id", "_lang", "lang"]);
    let error = refused(Target::new(&metadata, None).unwrap().validate());

    assert!(
        error.contains("\"_lang\": field names cannot be empty or start with `_` — rename the column with `--rename _lang=<name>`"),
        "{error}"
    );
}

#[test]
fn plan_names_list_types_and_source_columns() {
    let mut table = table(&["_id", "_lang"]);
    table
        .columns
        .push(("tags".into(), FieldSpec::list(false, ListValueType::String)));
    let plan = Plan {
        collection: "books".into(),
        target: Target::new(&table, None).unwrap(),
    }
    .to_string();

    assert!(plan.contains("\n  lang ← _lang: text\n"), "{plan}");
    assert!(plan.contains("\n  tags: text_list\n"), "{plan}");
}

#[test]
fn plan_names_partition_filter_and_limit() {
    let mut target = Target::new(&table(&["_id", "lang"]), None).unwrap();
    target.partition = Some("lang".into());
    target.filter = Some("year > 2000".into());
    target.limit = Some(10);
    let plan = Plan {
        collection: "books".into(),
        target,
    }
    .to_string();

    assert!(
        plan.contains("\n  _id ← _id\n  partition ← lang\n  filter year > 2000\n  limit 10\n"),
        "{plan}"
    );
}

#[rstest]
#[case::key(Some("sku"), None, "sku")]
#[case::explicit_id(None, Some("title"), "title")]
#[case::override_key(Some("sku"), Some("title"), "title")]
fn catalog_id(#[case] key: Option<&str>, #[case] id: Option<&str>, #[case] expected: &str) {
    let metadata = Table {
        key: key.map(str::to_string),
        ..table(&["sku", "title"])
    };
    let target = Target::new(&metadata, id).unwrap();
    assert_eq!(target.id, expected);
    assert!(!target.fields.contains_key(expected));
}

#[rstest]
#[case::one_candidate(&["event_id", "tenant"], "pass --id event_id")]
#[case::two_candidates(&["event_id", "user_id"], "pass --id <column>")]
#[case::no_candidate(&["sku", "title"], "pass --id <column>")]
fn missing_id_names_a_candidate(#[case] columns: &[&str], #[case] hint: &str) {
    let error = refused(Target::new(&table(columns), None));
    assert!(error.contains(hint), "{error}");
}

#[test]
fn unresolved_id_rejected() {
    let metadata = table(&["sku", "title"]);
    assert!(matches!(
        Target::new(&metadata, None),
        Err(topk::import::Error::MissingId { from, columns }) if from.value() == "books" && columns == ["sku", "title"]
    ));
}

#[test]
fn dotted_name_under_struct_rejected() {
    let mut target = Target::new(&table(&["_id", "meta"]), None).unwrap();
    target.fields["meta"] = Field::new(FieldSpec::r#struct(
        false,
        Vec::<(String, FieldSpec)>::new(),
    ));
    target
        .fields
        .insert("meta.lang".into(), Field::new(FieldSpec::text(false)));
    let error = refused(target.validate());
    assert!(
        error.contains("\"meta.lang\" conflicts with struct field \"meta\""),
        "{error}"
    );
}
