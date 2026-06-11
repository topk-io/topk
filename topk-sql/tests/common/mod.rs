#![allow(unused_imports)]

use topk_rs::proto::v1::data::Document;

mod context;
pub use context::{BooksContext, Scope, SessionContext, TableScope};

mod client;

#[allow(dead_code)]
pub fn ids<'a>(docs: impl IntoIterator<Item = &'a Document>) -> std::collections::HashSet<&'a str> {
    docs.into_iter().map(|doc| doc.id().unwrap()).collect()
}

#[macro_export]
macro_rules! ids {
    ($($v:expr),* $(,)?) => {
        ::std::collections::HashSet::from_iter([$($v),*])
    };
}
