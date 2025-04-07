use napi::bindgen_prelude::{check_status, FromNapiValue, Null, ToNapiValue};
use napi_derive::napi;
use std::{collections::HashMap, ptr};

use super::value::Value;

pub struct Document {
    fields: HashMap<String, Value>,
}

impl From<Document> for topk_protos::v1::data::Document {
    fn from(doc: Document) -> Self {
        topk_protos::v1::data::Document {
            fields: doc.fields.into_iter().map(|(k, v)| (k, v.into())).collect(),
        }
    }
}

impl From<topk_protos::v1::data::Document> for Document {
    fn from(doc: topk_protos::v1::data::Document) -> Self {
        Document {
            fields: doc.fields.into_iter().map(|(k, v)| (k, v.into())).collect(),
        }
    }
}

pub struct DocumentWrapper(pub topk_protos::v1::data::Document);

impl From<topk_protos::v1::data::Document> for DocumentWrapper {
    fn from(doc: topk_protos::v1::data::Document) -> Self {
        Self(doc)
    }
}

impl From<DocumentWrapper> for HashMap<String, Value> {
    fn from(wrapper: DocumentWrapper) -> Self {
        wrapper
            .0
            .fields
            .into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect()
    }
}
