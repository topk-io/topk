use super::value::Value;
use std::collections::HashMap;

pub struct Document(topk_rs::proto::v1::data::Document);

impl Document {
    pub fn new(doc: HashMap<String, Value>) -> Self {
        Self(topk_rs::proto::v1::data::Document {
            fields: doc.into_iter().map(|(k, v)| (k, v.into())).collect(),
        })
    }
}

impl From<topk_rs::proto::v1::data::Document> for Document {
    fn from(doc: topk_rs::proto::v1::data::Document) -> Self {
        Self(doc)
    }
}

impl From<Document> for HashMap<String, Value> {
    fn from(wrapper: Document) -> Self {
        wrapper
            .0
            .fields
            .into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect()
    }
}

impl From<Document> for topk_rs::proto::v1::data::Document {
    fn from(wrapper: Document) -> Self {
        topk_rs::proto::v1::data::Document {
            fields: wrapper
                .0
                .fields
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
        }
    }
}
