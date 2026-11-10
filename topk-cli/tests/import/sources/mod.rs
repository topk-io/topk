use ::topk::import::{build_row, Cursor, Uri};
use futures::{stream, StreamExt};
use serde_json::{json, Value};
use test_macros::rstest_ctx;
use topk_rs::doc;
use topk_rs::proto::v1::data::Value as Cell;

use crate::import::common::seed::Backend;
use crate::import::common::*;

mod elasticsearch;
mod files;
mod mongo;
mod postgres;

#[rstest_ctx(Scratch)]
#[case::sqlite_key(Backend::Sqlite)]
#[case::postgres_key(Backend::Postgres)]
#[case::mysql_key(Backend::Mysql)]
#[case::mongo_bson(Backend::Mongo)]
#[case::elasticsearch_pit(Backend::Elasticsearch)]
#[case::parquet_offset(Backend::Parquet)]
#[case::s3_offset(Backend::S3)]
async fn resumes_from_saved_cursor(_ctx: &mut Scratch, #[case] backend: Backend) {
    let count: usize = if matches!(backend, Backend::Mongo | Backend::Elasticsearch) {
        1001
    } else {
        4097
    };
    let mut expected: Vec<_> = (0..count)
        .map(|i| match backend {
            Backend::Mongo => i.to_string(),
            _ => format!("{i:05}"),
        })
        .collect();
    let source = backend.seed(expected.iter().enumerate().map(|(i, id)| doc!(
        "_id" => if matches!(backend, Backend::Mongo) { Cell::i64(i as i64) } else { Cell::string(id.clone()) },
        "n" => i as i64
    )).collect()).await;
    for (key, value) in source.env {
        std::env::set_var(key, value);
    }
    let target = target(&source.from, "_id", "n = { type = 'int' }");
    let uri = source
        .url
        .as_deref()
        .unwrap_or(&source.from)
        .parse::<Uri>()
        .unwrap();
    let reader = uri.build(&config()).await.unwrap();
    let mut scan = reader.scan(&target, None).unwrap();
    let first = scan.next().await.unwrap().unwrap();
    let consumed = first.rows.len() as u64;
    assert!(
        consumed > 0 && consumed < count as u64,
        "must cross a chunk boundary: {consumed}/{count}"
    );
    let cursor = first.cursor.clone().unwrap();
    match (&backend, &cursor) {
        (Backend::Elasticsearch, Cursor::Page { pit, sort }) => {
            assert!(!pit.is_empty() && !sort.is_empty())
        }
        (Backend::Mongo, Cursor::Key(key)) => assert_eq!(
            serde_json::from_str::<Value>(key).unwrap(),
            json!({"$numberLong": (consumed - 1).to_string()})
        ),
        (Backend::Parquet | Backend::S3, Cursor::Offset { rows, .. }) => {
            assert_eq!(*rows, consumed)
        }
        (Backend::Sqlite | Backend::Postgres | Backend::Mysql, Cursor::Key(key)) => {
            assert_eq!(key, &format!("{:05}", consumed - 1))
        }
        _ => panic!("wrong cursor for {backend:?}: {cursor:?}"),
    }
    drop(scan);
    let mut chunks = stream::iter([Ok(first)]).chain(reader.scan(&target, Some(cursor)).unwrap());
    let mut ids = Vec::new();
    while let Some(chunk) = chunks.next().await {
        let chunk = chunk.unwrap();
        for row in chunk.rows {
            ids.push(
                build_row(&target, row.unwrap()).unwrap().1.fields["_id"]
                    .as_string()
                    .unwrap()
                    .to_string(),
            );
        }
    }
    if matches!(backend, Backend::Elasticsearch) {
        ids.sort();
        expected.sort();
    }
    assert_eq!(ids, expected);
}
