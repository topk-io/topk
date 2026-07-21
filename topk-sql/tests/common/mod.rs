#![allow(unused_imports)]

use topk_rs::proto::v1::data::Document;

mod context;
pub use context::{BooksContext, Scope, SessionContext, TableScope};

mod client;

#[allow(dead_code)]
pub fn ids<'a>(docs: impl IntoIterator<Item = &'a Document>) -> std::collections::HashSet<&'a str> {
    docs.into_iter().map(|doc| doc.id().unwrap()).collect()
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
