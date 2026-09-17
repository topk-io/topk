#![allow(unused_imports)]

use std::time::{Duration, Instant};

use topk_rs::proto::v1::data::Document;

mod context;
pub use context::{BooksContext, Scope, SessionContext, TableScope};

mod client;

#[allow(dead_code)]
pub fn ids<'a>(docs: impl IntoIterator<Item = &'a Document>) -> std::collections::HashSet<&'a str> {
    docs.into_iter().map(|doc| doc.id().unwrap()).collect()
}

#[allow(dead_code)]
pub async fn wait_for_index(ctx: &TableScope, sql: &str) -> anyhow::Result<Vec<Document>> {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        match ctx.sql(sql).await {
            Ok(rows) => return Ok(rows),
            Err(error)
                if Instant::now() < deadline
                    && error
                        .downcast_ref::<sqlx::Error>()
                        .and_then(|e| e.as_database_error())
                        .and_then(|e| e.code())
                        .is_some_and(|code| code == "55000") => {}
            Err(error) => return Err(error),
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

#[track_caller]
#[allow(dead_code)]
pub fn assert_rows_eq_unordered(mut actual: Vec<Document>, mut expected: Vec<Document>) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "row count mismatch: {actual:?} vs {expected:?}"
    );
    for want in expected.drain(..) {
        let pos = actual.iter().position(|got| got == &want);
        match pos {
            Some(i) => {
                actual.remove(i);
            }
            None => panic!("expected row {want:?} not found in actual rows: {actual:?}"),
        }
    }
}

#[macro_export]
macro_rules! ids {
    ($($v:expr),* $(,)?) => {
        ::std::collections::HashSet::from_iter([$($v),*])
    };
}
