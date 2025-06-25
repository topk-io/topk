use std::collections::HashMap;

use super::value::Value;

pub struct NapiDocument(topk_rs::proto::v1::data::Document);

impl From<topk_rs::proto::v1::data::Document> for NapiDocument {
    fn from(doc: topk_rs::proto::v1::data::Document) -> Self {
        Self(doc)
    }
}

impl From<NapiDocument> for HashMap<String, Value> {
    fn from(wrapper: NapiDocument) -> Self {
        wrapper
            .0
            .fields
            .into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect()
    }
}
