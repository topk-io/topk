use futures::StreamExt;
use mongodb::bson::{doc, oid::ObjectId};
use serde_json::json;
use topk::import::{build_row, Cursor, Uri};

use crate::import::common::seed::mongo;
use crate::import::common::*;

#[tokio::test]
async fn mongo_binary() {
    let name = unique_name("bson");
    mongo::Mongo::client()
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
    let catalog = crate::import::common::catalog_of(mongo::Mongo::URL)
        .await
        .unwrap();
    let target = topk::import::discover(&catalog, &[name], None, None)
        .unwrap()
        .collections
        .into_values()
        .next()
        .unwrap();

    let docs = stream(Some(mongo::Mongo::URL), &target).await.unwrap();

    assert_eq!(
        docs,
        [(None, json!({"_id": "b1", "bin": [222, 173, 190, 239]}))]
    );
}

/// Mongo sorts `_id` across BSON types (numbers, then strings, then ObjectIds),
/// so a resume after any id must still reach every id that sorts after it.
#[tokio::test]
async fn mongo_resume_crosses_id_types() {
    let name = unique_name("mixed");
    mongo::Mongo::client()
        .database("demo")
        .collection::<mongodb::bson::Document>(&name)
        .insert_many([
            doc! { "_id": 1_i64, "n": 1 },
            doc! { "_id": 2_i64, "n": 2 },
            doc! { "_id": 3_i64, "n": 3 },
            doc! { "_id": "a", "n": 4 },
            doc! { "_id": "b", "n": 5 },
            doc! { "_id": ObjectId::parse_str("65f000000000000000000000").unwrap(), "n": 6 },
        ])
        .await
        .unwrap();
    let source = mongo::Mongo::URL
        .parse::<Uri>()
        .unwrap()
        .build(&config())
        .await
        .unwrap();
    let target = target(&name, "_id", "n = { type = 'int' }");
    let ids = |after: Option<Cursor>| {
        let mut scan = source.scan(&target, after).unwrap();
        let target = &target;
        async move {
            let mut ids = Vec::new();
            while let Some(chunk) = scan.next().await {
                for row in chunk.unwrap().rows {
                    let doc = build_row(target, row.unwrap()).unwrap().1;
                    ids.push(doc.fields["_id"].as_string().unwrap().to_string());
                }
            }
            ids
        }
    };

    let all = ids(None).await;
    let resumed = ids(Some(Cursor::Key(r#"{"$numberLong":"2"}"#.into()))).await;

    assert_eq!(all, ["1", "2", "3", "a", "b", "65f000000000000000000000"]);
    assert_eq!(resumed, all[2..]);
}
