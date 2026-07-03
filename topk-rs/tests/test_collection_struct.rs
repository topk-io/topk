use std::collections::HashMap;
use test_context::test_context;
use topk_rs::doc;
use topk_rs::error::{DocumentValidationError, SchemaValidationError, ValidationErrorBag};
use topk_rs::proto::v1::control::{FieldIndex, FieldSpec};
use topk_rs::proto::v1::data::Value;
use topk_rs::query::{field, fns, select};
use topk_rs::Error;

mod utils;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_struct_round_trip(ctx: &mut ProjectTestContext) {
    // 2-level nested struct: outer.inner.{leaf, sibling}
    let collection = ctx
        .client
        .collections()
        .create(
            ctx.wrap("test"),
            HashMap::from_iter([(
                "outer".to_string(),
                FieldSpec::r#struct(
                    false,
                    [(
                        "inner",
                        FieldSpec::r#struct(
                            false,
                            [
                                ("leaf", FieldSpec::text(false)),
                                ("sibling", FieldSpec::text(false)),
                            ],
                        ),
                    )],
                ),
            )]),
            None,
        )
        .await
        .expect("could not create collection");

    let lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![doc!(
            "_id" => "one",
            "outer" => Value::r#struct([(
                "inner",
                Value::r#struct([("leaf", "v".into()), ("sibling", "s".into())]),
            )]),
        )])
        .await
        .expect("could not upsert");

    let docs = ctx
        .client
        .collection(&collection.name)
        .get(["one"], None, Some(lsn), None)
        .await
        .expect("could not get document");

    let one = docs.get("one").expect("missing doc");
    let inner = one
        .get("outer")
        .and_then(|v| v.as_struct())
        .and_then(|s| s.get("inner"))
        .and_then(|v| v.as_struct())
        .expect("inner should be nested struct");
    assert_eq!(inner.get("leaf").and_then(|v| v.as_string()), Some("v"));
    assert_eq!(inner.get("sibling").and_then(|v| v.as_string()), Some("s"));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_struct_query(ctx: &mut ProjectTestContext) {
    // select(dotted) + filter(dotted) + fetch(dotted) in one query.
    let collection = ctx
        .client
        .collections()
        .create(
            ctx.wrap("test"),
            HashMap::from_iter([(
                "meta".to_string(),
                FieldSpec::r#struct(
                    false,
                    [
                        ("author", FieldSpec::text(false)),
                        ("year", FieldSpec::integer(false)),
                        ("tag", FieldSpec::text(false)),
                    ],
                ),
            )]),
            None,
        )
        .await
        .expect("could not create collection");

    ctx.client
        .collection(&collection.name)
        .upsert(vec![
            doc!(
                "_id" => "old",
                "meta" => Value::r#struct([
                    ("author", "alice".into()),
                    ("year", 1999i64.into()),
                    ("tag", "classic".into()),
                ]),
            ),
            doc!(
                "_id" => "new",
                "meta" => Value::r#struct([
                    ("author", "bob".into()),
                    ("year", 2024i64.into()),
                    ("tag", "fresh".into()),
                ]),
            ),
        ])
        .await
        .expect("could not upsert");

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("meta.author", field("meta.author"))])
                .filter(field("meta.year").gt(2020i64))
                .sort(field("meta.year"), true)
                .limit(10)
                .fetch(["meta.tag"]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id().unwrap(), "new");
    let fields = &results[0].fields;
    assert_eq!(
        fields.get("meta.author").and_then(|v| v.as_string()),
        Some("bob"),
    );
    assert_eq!(
        fields.get("meta.tag").and_then(|v| v.as_string()),
        Some("fresh"),
    );
    assert!(fields.get("meta.year").is_none());
    assert!(fields.get("meta").is_none());
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_struct_semantic_index_on_sub_field(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(
            ctx.wrap("test"),
            HashMap::from_iter([(
                "meta".to_string(),
                FieldSpec::r#struct(
                    false,
                    [(
                        "description",
                        FieldSpec::text(false).with_index(FieldIndex::semantic()),
                    )],
                ),
            )]),
            None,
        )
        .await
        .expect("could not create collection");

    let lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![
            doc!(
                "_id" => "rust",
                "meta" => Value::r#struct([("description", "a systems programming language".into())]),
            ),
            doc!(
                "_id" => "python",
                "meta" => Value::r#struct([("description", "a snake".into())]),
            ),
        ])
        .await
        .expect("could not upsert");

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([(
                "sim",
                fns::semantic_similarity("meta.description", "programming"),
            )])
            .sort(field("sim"), true)
            .limit(2),
            Some(lsn),
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(results.len(), 2);
    for doc in &results {
        assert!(
            doc.fields.get("sim").is_some(),
            "missing sim score: {doc:?}"
        );
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_struct_update(ctx: &mut ProjectTestContext) {
    // Partial sub-field update must preserve siblings (deep merge).
    let collection = ctx
        .client
        .collections()
        .create(
            ctx.wrap("test"),
            HashMap::from_iter([(
                "meta".to_string(),
                FieldSpec::r#struct(
                    false,
                    [
                        ("author", FieldSpec::text(false)),
                        ("title", FieldSpec::text(false)),
                    ],
                ),
            )]),
            None,
        )
        .await
        .expect("could not create collection");

    ctx.client
        .collection(&collection.name)
        .upsert(vec![doc!(
            "_id" => "one",
            "meta" => Value::r#struct([("author", "alice".into()), ("title", "v1".into())]),
        )])
        .await
        .expect("could not upsert");

    let lsn = ctx
        .client
        .collection(&collection.name)
        .update(
            vec![doc!(
                "_id" => "one",
                "meta" => Value::r#struct([("title", "v2".into())]),
            )],
            true,
        )
        .await
        .expect("could not update");

    let docs = ctx
        .client
        .collection(&collection.name)
        .get(["one"], None, Some(lsn), None)
        .await
        .expect("could not get document");

    let meta = docs
        .get("one")
        .and_then(|d| d.get("meta"))
        .and_then(|v| v.as_struct())
        .expect("expected nested struct");
    assert_eq!(meta.get("title").and_then(|v| v.as_string()), Some("v2"));
    // Sibling sub-field set at upsert and not mentioned in update — must survive.
    assert_eq!(
        meta.get("author").and_then(|v| v.as_string()),
        Some("alice")
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_literal_dotted_field_roundtrip(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default(), None)
        .await
        .expect("could not create collection");

    let lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![doc!(
            "_id" => "one",
            "meta.foo" => 3i64,
        )])
        .await
        .expect("could not upsert");

    let docs = ctx
        .client
        .collection(&collection.name)
        .get(["one"], None, Some(lsn.clone()), None)
        .await
        .expect("could not get document");
    let one = docs.get("one").expect("missing doc");
    assert_eq!(one.get("meta.foo").and_then(|v| v.as_i64()), Some(3));
    assert!(one.get("meta").is_none());

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("meta.foo", field("meta.foo"))])
                .sort(field("meta.foo"), true)
                .limit(10),
            Some(lsn),
            None,
        )
        .await
        .expect("could not query");
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].fields.get("meta.foo").and_then(|v| v.as_i64()),
        Some(3),
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_upsert_mixed_dotted_and_struct_rejected(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default(), None)
        .await
        .expect("could not create collection");

    let err = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![doc!(
            "_id" => "one",
            "meta.foo" => 3i64,
            "meta" => Value::r#struct([("bar", 4i64.into())]),
        )])
        .await
        .expect_err("mixing literal dotted key with a struct sibling should fail");

    assert!(
        matches!(
            err,
            Error::DocumentValidationError(ref s) if s == &ValidationErrorBag::from(vec![
                DocumentValidationError::InvalidFieldName {
                    doc_id: "one".to_string(),
                    field: "meta.foo".to_string(),
                },
            ])
        ),
        "got: {err:?}",
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_create_schema_struct_dotted_sub_field_rejected(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .collections()
        .create(
            ctx.wrap("test"),
            HashMap::from_iter([(
                "meta".to_string(),
                FieldSpec::r#struct(false, [("a.b", FieldSpec::text(false))]),
            )]),
            None,
        )
        .await
        .expect_err("schema with dotted sub-field name should fail");

    assert!(
        matches!(
            err,
            Error::SchemaValidationError(ref bag) if bag.iter().any(|e| matches!(
                e,
                SchemaValidationError::FieldNameContainsDot { field } if field == "meta.a.b",
            ))
        ),
        "got: {err:?}",
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_upsert_struct_dotted_sub_field_rejected(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default(), None)
        .await
        .expect("could not create collection");

    let err = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![doc!(
            "_id" => "one",
            "meta" => Value::r#struct([("a.b", "v".into())]),
        )])
        .await
        .expect_err("dotted sub-field name should fail");

    assert!(
        matches!(
            err,
            Error::DocumentValidationError(ref s) if s == &ValidationErrorBag::from(vec![
                DocumentValidationError::InvalidFieldName {
                    doc_id: "one".to_string(),
                    field: "meta.a.b".to_string(),
                },
            ])
        ),
        "got: {err:?}",
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_struct_get_all_fields(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(
            ctx.wrap("test"),
            HashMap::from_iter([(
                "meta".to_string(),
                FieldSpec::r#struct(
                    false,
                    [
                        ("author", FieldSpec::text(false)),
                        ("year", FieldSpec::integer(false)),
                    ],
                ),
            )]),
            None,
        )
        .await
        .expect("could not create collection");

    let lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![doc!(
            "_id" => "one",
            "meta" => Value::r#struct([
                ("author", "alice".into()),
                ("year", 2024i64.into()),
            ]),
        )])
        .await
        .expect("could not upsert");

    let docs = ctx
        .client
        .collection(&collection.name)
        .get(["one"], None, Some(lsn), None)
        .await
        .expect("could not get document");
    let one = docs.get("one").expect("missing doc");

    let meta = one.get("meta").and_then(|v| v.as_struct()).unwrap();
    assert_eq!(
        meta.get("author").and_then(|v| v.as_string()),
        Some("alice"),
    );
    assert_eq!(meta.get("year").and_then(|v| v.as_i64()), Some(2024));
    assert!(one.get("meta.author").is_none());
    assert!(one.get("meta.year").is_none());
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_struct_get_returns_whole_struct(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(
            ctx.wrap("test"),
            HashMap::from_iter([(
                "meta".to_string(),
                FieldSpec::r#struct(
                    false,
                    [
                        ("author", FieldSpec::text(false)),
                        ("year", FieldSpec::integer(false)),
                    ],
                ),
            )]),
            None,
        )
        .await
        .expect("could not create collection");

    let lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![doc!(
            "_id" => "one",
            "meta" => Value::r#struct([
                ("author", "alice".into()),
                ("year", 2024i64.into()),
            ]),
        )])
        .await
        .expect("could not upsert");

    let docs = ctx
        .client
        .collection(&collection.name)
        .get(["one"], Some(vec!["meta".to_string()]), Some(lsn), None)
        .await
        .expect("could not get document");
    let one = docs.get("one").expect("missing doc");

    let meta = one.get("meta").and_then(|v| v.as_struct()).unwrap();
    assert_eq!(
        meta.get("author").and_then(|v| v.as_string()),
        Some("alice"),
    );
    assert_eq!(meta.get("year").and_then(|v| v.as_i64()), Some(2024));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_struct_get_returns_flat_leaf(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(
            ctx.wrap("test"),
            HashMap::from_iter([(
                "meta".to_string(),
                FieldSpec::r#struct(
                    false,
                    [
                        ("author", FieldSpec::text(false)),
                        ("year", FieldSpec::integer(false)),
                    ],
                ),
            )]),
            None,
        )
        .await
        .expect("could not create collection");

    let lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![doc!(
            "_id" => "one",
            "meta" => Value::r#struct([
                ("author", "alice".into()),
                ("year", 2024i64.into()),
            ]),
        )])
        .await
        .expect("could not upsert");

    let docs = ctx
        .client
        .collection(&collection.name)
        .get(
            ["one"],
            Some(vec!["meta.author".to_string()]),
            Some(lsn),
            None,
        )
        .await
        .expect("could not get document");
    let one = docs.get("one").expect("missing doc");

    assert_eq!(
        one.get("meta.author").and_then(|v| v.as_string()),
        Some("alice")
    );
    assert!(one.get("meta").is_none());
    assert!(one.get("meta.year").is_none());
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_struct_underscore_sub_field(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(
            ctx.wrap("test"),
            HashMap::from_iter([(
                "meta".to_string(),
                FieldSpec::r#struct(false, [("_bar", FieldSpec::text(false))]),
            )]),
            None,
        )
        .await
        .expect("could not create collection");

    // Upsert document
    let lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![doc!(
            "_id" => "one",
            "meta" => Value::r#struct([("_bar", "v".into())]),
        )])
        .await
        .expect("could not upsert");

    // Request the whole struct
    let docs = ctx
        .client
        .collection(&collection.name)
        .get(
            ["one"],
            Some(vec!["meta".to_string()]),
            Some(lsn.clone()),
            None,
        )
        .await
        .expect("could not get document");
    let meta = docs
        .get("one")
        .and_then(|d| d.get("meta"))
        .and_then(|v| v.as_struct())
        .expect("missing meta struct");
    assert_eq!(meta.get("_bar").and_then(|v| v.as_string()), Some("v"));

    // Request the dotted leaf
    let docs = ctx
        .client
        .collection(&collection.name)
        .get(
            ["one"],
            Some(vec!["meta._bar".to_string()]),
            Some(lsn),
            None,
        )
        .await
        .expect("could not get document");
    let one = docs.get("one").expect("missing doc");
    assert_eq!(one.get("meta._bar").and_then(|v| v.as_string()), Some("v"));
    assert!(one.get("meta").is_none());
}
