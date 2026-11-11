use crate::import::common::refused;
use rstest::rstest;
use topk::import::source::Catalog;
use topk::import::source::Table;
use topk_rs::proto::v1::control::FieldSpec;

fn catalog(tables: impl IntoIterator<Item = Table>) -> Catalog {
    Catalog {
        tables: tables.into_iter().collect(),
    }
}

fn table(columns: &[&str]) -> Table {
    Table {
        from: "books".into(),
        collection_hint: Some("books".into()),
        key: None,
        columns: columns
            .iter()
            .map(|name| ((*name).into(), Some(FieldSpec::text(false))))
            .collect(),
    }
}

/// Field names may not start with `_`, but sources use that for their own
/// bookkeeping columns — so a discovered spec renames them and runs as written.
#[test]
fn underscore_columns_are_renamed() {
    let catalog = catalog([table(&["_id", "_lang", "_n_chars", "text"])]);
    let spec = catalog.discover(&[], None, None).unwrap();
    let target = spec.collections.values().next().unwrap();

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
    let catalog = catalog([table(&["_id", "_lang", "lang"])]);
    let error = refused(catalog.discover(&[], None, None));

    assert!(
        error.contains("\"_lang\": field names cannot be empty or start with `_`"),
        "{error}"
    );
}

#[test]
fn all_objects_skipped_says_so() {
    let catalog = catalog([table(&["id"])]);
    let error = refused(catalog.discover(&[], None, None));

    assert!(
        error.contains("all 1 matched object(s) were skipped"),
        "{error}"
    );
}

#[test]
fn missing_hint_requires_name() {
    let catalog = catalog([Table {
        collection_hint: None,
        ..table(&["id", "title"])
    }]);
    let message = refused(catalog.discover(&[], None, None));
    assert!(message.contains("pass --to <name>"), "got: {message}");

    let spec = catalog.discover(&[], Some("parts"), None).unwrap();
    assert_eq!(spec.collections.keys().collect::<Vec<_>>(), ["parts"]);
    assert_eq!(spec.collections["parts"].from, "books");
}

#[rstest]
#[case::exact_name("public.books", "books")]
#[case::qualified_glob("public.book*", "books")]
#[case::inline_rename("public.books=renamed", "renamed")]
fn catalog_selection(#[case] pattern: &str, #[case] name: &str) {
    let catalog = catalog([
        Table {
            from: "public.books".into(),
            ..table(&["id", "title"])
        },
        Table {
            from: "public.authors".into(),
            collection_hint: Some("authors".into()),
            ..table(&["id", "name"])
        },
    ]);
    let spec = catalog.discover(&[pattern.into()], None, None).unwrap();
    assert_eq!(spec.collections.keys().collect::<Vec<_>>(), [name]);
    assert_eq!(spec.collections[name].from, "public.books");
}

#[rstest]
#[case::no_match("missing*", &["nothing to import", "object(s)"])]
#[case::schema_collision("books", &["rename one inline"])]
#[case::normalized_collision("public.books?x", &["both map to collection"])]
fn selection_errors(#[case] pattern: &str, #[case] fragments: &[&str]) {
    let catalog = catalog(
        [
            ("public.books", "books"),
            ("other.books", "books"),
            ("public.books x", "books x"),
            ("public.books_x", "books_x"),
        ]
        .into_iter()
        .map(|(from, hint)| Table {
            from: from.into(),
            collection_hint: Some(hint.into()),
            ..table(&["id", "title"])
        }),
    );
    let message = refused(catalog.discover(&[pattern.into()], None, None));
    for fragment in fragments {
        assert!(message.contains(fragment), "{message}");
    }
}

#[rstest]
#[case::key(Some("sku"), None, "sku")]
#[case::explicit_id(None, Some("title"), "title")]
#[case::override_key(Some("sku"), Some("title"), "title")]
fn catalog_id(#[case] key: Option<&str>, #[case] id: Option<&str>, #[case] expected: &str) {
    let catalog = catalog([Table {
        key: key.map(str::to_string),
        ..table(&["sku", "title"])
    }]);
    let spec = catalog.discover(&[], None, id).unwrap();
    assert_eq!(spec.collections["books"].id, expected);
    assert!(!spec.collections["books"].fields.contains_key(expected));
}

#[test]
fn unresolved_id_rejected() {
    let catalog = catalog([table(&["sku", "title"])]);
    assert!(matches!(
        catalog.discover(&[], None, None),
        Err(topk::import::Error::MissingId(name)) if name == "books"
    ));
}

#[test]
fn unresolved_objects_skipped() {
    let catalog = catalog([
        table(&["sku", "title"]),
        Table {
            from: "authors".into(),
            collection_hint: Some("authors".into()),
            ..table(&["id", "name"])
        },
    ]);
    let spec = catalog.discover(&[], None, None).unwrap();
    assert_eq!(spec.collections.keys().collect::<Vec<_>>(), ["authors"]);
}
