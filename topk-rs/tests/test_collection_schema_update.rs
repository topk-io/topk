use std::collections::HashMap;

use test_context::test_context;
use topk_rs::{
    doc,
    proto::v1::{
        control::{Collection, FieldIndex, FieldSpec, KeywordIndexType},
        data::Document,
    },
    query::{field, filter, r#match},
    schema, Error,
};

mod utils;
use utils::ProjectTestContext;

fn keyword(spec: FieldSpec) -> FieldSpec {
    spec.with_index(FieldIndex::keyword(KeywordIndexType::Text))
}

async fn create(ctx: &mut ProjectTestContext, schema: HashMap<String, FieldSpec>) -> Collection {
    ctx.client
        .collections()
        .create(ctx.wrap("books"), schema, None)
        .await
        .expect("could not create collection")
}

fn docs() -> Vec<Document> {
    vec![
        doc!("_id" => "pride", "title" => "Pride and Prejudice", "summary" => "a love story in georgian england", "rating" => 5u32),
        doc!("_id" => "gatsby", "title" => "The Great Gatsby", "summary" => "love and loss in the jazz age", "rating" => 4u32),
        doc!("_id" => "moby", "title" => "Moby Dick", "summary" => "a whale and a captain", "rating" => 3u32),
    ]
}

async fn ids_matching(
    ctx: &ProjectTestContext,
    collection: &str,
    term: &str,
    fieldname: &str,
    lsn: Option<String>,
) -> Result<Vec<String>, Error> {
    let docs = ctx
        .client
        .collection(collection)
        .query(
            filter(r#match(term, Some(fieldname), None, false))
                .select([("title", field("title"))])
                .limit(100),
            lsn,
            None,
        )
        .await?;
    let mut ids: Vec<String> = docs.iter().map(|d| d.id().unwrap().to_string()).collect();
    ids.sort();
    Ok(ids)
}

/// `required` is accepted when every stored document has the field and rejected, naming the
/// field, when one lacks it. A rejected update leaves the schema untouched.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_required_is_proven_against_data(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, schema!("title" => FieldSpec::text(true))).await;
    let name = collection.name.clone();
    ctx.client.collection(&name).upsert(docs()).await.unwrap();

    let updated = ctx
        .client
        .collections()
        .update(&name, schema!("rating" => FieldSpec::integer(true)), vec![])
        .await
        .expect("every document has a rating");
    assert!(updated.schema["rating"].required);

    let err = ctx
        .client
        .collection(&name)
        .upsert(vec![doc!("_id" => "x", "title" => "no rating")])
        .await
        .unwrap_err();
    assert!(matches!(err, Error::DocumentValidationError(_)), "{err:?}");

    let err = ctx
        .client
        .collections()
        .update(&name, schema!("isbn" => FieldSpec::text(true)), vec![])
        .await
        .unwrap_err();
    match err {
        Error::Unexpected(msg) => assert!(msg.contains("isbn"), "{msg}"),
        e => panic!("{e:?}"),
    }
    let fetched = ctx.client.collections().get(&name).await.unwrap();
    assert_eq!(fetched.schema, updated.schema);

    // a type the data contradicts is rejected too
    let err = ctx
        .client
        .collections()
        .update(&name, schema!("summary" => FieldSpec::integer(false)), vec![])
        .await
        .unwrap_err();
    match err {
        Error::Unexpected(msg) => assert!(msg.contains("summary"), "{msg}"),
        e => panic!("{e:?}"),
    }
}

/// Dropping an unindexed field keeps its data filterable; index changes are rejected.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_drop_field_and_index_changes(ctx: &mut ProjectTestContext) {
    let collection = create(
        ctx,
        schema!(
            "title" => keyword(FieldSpec::text(true)),
            "summary" => FieldSpec::text(false),
        ),
    )
    .await;
    let name = collection.name.clone();
    ctx.client.collection(&name).upsert(docs()).await.unwrap();

    let updated = ctx
        .client
        .collections()
        .update(&name, HashMap::new(), vec!["summary".to_string()])
        .await
        .unwrap();
    assert!(!updated.schema.contains_key("summary"));

    let docs = ctx
        .client
        .collection(&name)
        .query(
            filter(field("summary").eq("a whale and a captain")).limit(10),
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].id().unwrap(), "moby");

    for (schema, drop_fields) in [
        (schema!("summary" => keyword(FieldSpec::text(false))), vec![]),
        (schema!("title" => FieldSpec::text(true)), vec![]),
        (HashMap::new(), vec!["title".to_string()]),
    ] {
        let err = ctx
            .client
            .collections()
            .update(&name, schema, drop_fields)
            .await
            .unwrap_err();
        match err {
            Error::Unexpected(msg) => assert!(msg.contains("index"), "{msg}"),
            e => panic!("{e:?}"),
        }
    }
    let fetched = ctx.client.collections().get(&name).await.unwrap();
    assert_eq!(fetched.schema, updated.schema);
}
