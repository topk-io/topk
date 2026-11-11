use futures::TryStreamExt;
use mongodb::bson::{doc, Bson, Document};
use serde_json::json;
use test_macros::rstest_ctx;
use topk::import::source::{Checkpoint, Cursor, ReadEvent};
use topk_rs::proto::v1::control::FieldSpec;

use crate::import::common::seed::mongo;
use crate::import::common::*;

#[rstest_ctx(Scratch)]
async fn mongo_binary(ctx: &Scratch) {
    let name = ctx.name("bson");
    mongo::client()
        .await
        .database("demo")
        .collection::<mongodb::bson::Document>(&name)
        .insert_one(mongodb::bson::doc! {
            "_id": "b1",
            "bin": mongodb::bson::Binary {
                subtype: mongodb::bson::spec::BinarySubtype::Generic,
                bytes: vec![0xde, 0xad, 0xbe, 0xef],
            },
        })
        .await
        .unwrap();
    let source = source(mongo::URL).await.unwrap();
    let spec = source
        .catalog()
        .await
        .unwrap()
        .discover(&[name.clone()], None, None)
        .unwrap();
    let docs = json_documents(&source, &spec.collections[&name])
        .await
        .unwrap();

    assert_eq!(
        docs,
        [(None, json!({"_id": "b1", "bin": [222, 173, 190, 239]}))]
    );
}

#[rstest_ctx(Scratch)]
async fn resume_uses_native_id(ctx: &Scratch) {
    let name = ctx.name("resume");
    let docs: Vec<_> = (0..1001)
        .map(|n| {
            doc! {
                "_id": if n == 1000 { Bson::String("tail".into()) } else { Bson::Int32(n) },
                "external_id": "same",
                "n": n,
            }
        })
        .collect();
    mongo::client()
        .await
        .database("demo")
        .collection::<Document>(&name)
        .insert_many(docs)
        .await
        .unwrap();
    let source = source(mongo::URL).await.unwrap();
    let target = target(&name, "external_id", [("n", FieldSpec::integer(false))]);
    let mut scan = source.scan(&target, None).await.unwrap();
    let first = match scan.try_next().await.unwrap().unwrap() {
        ReadEvent::Chunk(chunk) => chunk,
        ReadEvent::Restarted => panic!("unexpected restart"),
    };
    assert_eq!(first.rows.len(), 1000);
    let checkpoint = Checkpoint {
        cursor: first.cursor.unwrap(),
        consumed: 1000,
    };
    assert!(matches!(&checkpoint.cursor, Cursor::Mongo(_)));
    drop(scan);
    let mut scan = source.scan(&target, Some(&checkpoint)).await.unwrap();
    let last = match scan.try_next().await.unwrap().unwrap() {
        ReadEvent::Chunk(chunk) => chunk,
        ReadEvent::Restarted => panic!("unexpected restart"),
    };
    assert_eq!(last.rows.len(), 1);
    let (_, document) = target
        .document(last.rows.into_iter().next().unwrap().unwrap())
        .unwrap();
    assert_eq!(
        serde_json::Value::Object(json(&document)),
        json!({"_id": "same", "n": 1000})
    );
    assert!(scan.try_next().await.unwrap().is_none());
}

#[rstest_ctx(Scratch)]
#[case::native_id("_id", true)]
#[case::custom_id("external_id", false)]
async fn legacy_resume_key(ctx: &Scratch, #[case] id: &str, #[case] accepted: bool) {
    let source = source(mongo::URL).await.unwrap();
    let target = target(&ctx.name("legacy"), id, [("n", FieldSpec::integer(false))]);
    let checkpoint = Checkpoint {
        cursor: Cursor::Key("42".into()),
        consumed: 1,
    };
    let result = source.scan(&target, Some(&checkpoint)).await;
    if accepted {
        assert!(result.is_ok());
    } else {
        assert!(refused(result).contains("legacy Mongo checkpoint"));
    }
}
