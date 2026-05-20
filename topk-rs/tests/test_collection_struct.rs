use std::collections::HashMap;
use test_context::test_context;
use topk_rs::doc;
use topk_rs::proto::v1::control::FieldSpec;
use topk_rs::proto::v1::data::Value;
use topk_rs::query::{field, fns, select};

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
                                ("leaf", FieldSpec::text(false, None)),
                                ("sibling", FieldSpec::text(false, None)),
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
                        ("author", FieldSpec::text(false, None)),
                        ("year", FieldSpec::integer(false)),
                        ("tag", FieldSpec::text(false, None)),
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
                .topk(field("meta.year"), 10, true)
                .fetch(["meta.tag"]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id().unwrap(), "new");
    let meta = results[0]
        .fields
        .get("meta")
        .and_then(|v| v.as_struct())
        .expect("meta should re-nest as struct");
    assert_eq!(meta.get("author").and_then(|v| v.as_string()), Some("bob"));
    assert_eq!(meta.get("tag").and_then(|v| v.as_string()), Some("fresh"));
    // We did not request `meta.year`; it should not be returned.
    assert!(
        meta.get("year").is_none(),
        "fetch should not over-return: {meta:?}"
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_struct_semantic_index_on_sub_field(ctx: &mut ProjectTestContext) {
    // Semantic index declared on a struct sub-field; query via dotted path.
    let collection = ctx
        .client
        .collections()
        .create(
            ctx.wrap("test"),
            HashMap::from_iter([(
                "meta".to_string(),
                FieldSpec::r#struct(false, [("description", FieldSpec::semantic(false))]),
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
            .topk(field("sim"), 2, true),
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
                        ("author", FieldSpec::text(false, None)),
                        ("title", FieldSpec::text(false, None)),
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
