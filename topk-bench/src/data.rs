use std::collections::HashMap;

use arrow_array::Int64Array;
use arrow_array::{
    types::Float64Type, Array, LargeListArray, LargeStringArray, PrimitiveArray, RecordBatch,
};
use prost::Message;

use topk_rs::proto::v1::data::{Document as RsDocument, Value as RsValue};

#[derive(Debug, Clone)]
pub struct Document {
    inner: RsDocument,
}

impl<'a> IntoIterator for &'a Document {
    type Item = (String, RsValue);
    type IntoIter = std::collections::hash_map::IntoIter<String, RsValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.fields.clone().into_iter()
    }
}

impl Document {
    pub fn new(fields: HashMap<String, RsValue>) -> Self {
        Self {
            inner: RsDocument { fields },
        }
    }

    pub fn get(&self, key: &str) -> Option<&RsValue> {
        self.inner.fields.get(key)
    }

    pub fn remove(&mut self, key: &str) -> Option<RsValue> {
        self.inner.fields.remove(key)
    }

    pub fn insert(&mut self, key: impl Into<String>, value: RsValue) {
        self.inner.fields.insert(key.into(), value);
    }

    pub fn encoded_len(&self) -> usize {
        self.inner.encoded_len()
    }
}

impl Into<RsDocument> for Document {
    fn into(self) -> RsDocument {
        self.inner
    }
}

impl From<RsDocument> for Document {
    fn from(inner: RsDocument) -> Self {
        Self { inner }
    }
}

pub fn parse_bench_01(batch: RecordBatch) -> Vec<Document> {
    let id = batch
        .column_by_name("id")
        .expect("id column not found")
        .as_any()
        .downcast_ref::<LargeStringArray>()
        .expect("id column is not a LargeStringArray");

    let text = batch
        .column_by_name("text")
        .expect("text column not found")
        .as_any()
        .downcast_ref::<LargeStringArray>()
        .expect("text column is not a LargeStringArray");

    let mut dense = {
        let list = batch
            .column_by_name("dense")
            .expect("dense column not found")
            .as_any()
            .downcast_ref::<LargeListArray>()
            .expect("dense column is not LargeList<Float64>");

        let mut out = Vec::with_capacity(list.len());
        for i in 0..list.len() {
            if list.is_null(i) {
                out.push(Vec::new());
                continue;
            }
            let sub = list.value(i); // each row’s vector
            let floats = sub
                .as_any()
                .downcast_ref::<PrimitiveArray<Float64Type>>()
                .expect("inner type not Float64Array");
            let vec: Vec<f32> = floats.values().iter().map(|v| *v as f32).collect();
            out.push(vec);
        }
        out
    };

    let numerical_filter = batch
        .column_by_name("numerical_filter")
        .expect("numerical_filter column not found")
        .as_any()
        .downcast_ref::<Int64Array>()
        .expect("numerical_filter column is not a Int64Array");

    let categorical_filter = batch
        .column_by_name("categorical_filter")
        .expect("categorical_filter column not found")
        .as_any()
        .downcast_ref::<LargeStringArray>()
        .expect("categorical_filter column is not a LargeStringArray");

    let mut rows = Vec::with_capacity(batch.num_rows());
    for i in 0..batch.num_rows() {
        let id = id.value(i).to_string();
        let text = text.value(i).to_string();
        let dense_embedding = std::mem::take(&mut dense[i]);
        let numerical_filter = numerical_filter.value(i) as u32;
        let categorical_filter = categorical_filter.value(i).to_string();

        rows.push(Document::new(HashMap::from([
            ("id".to_string(), RsValue::string(id)),
            ("text".to_string(), RsValue::string(text)),
            (
                "dense_embedding".to_string(),
                RsValue::list(dense_embedding),
            ),
            (
                "numerical_filter".to_string(),
                RsValue::u32(numerical_filter),
            ),
            (
                "categorical_filter".to_string(),
                RsValue::string(categorical_filter),
            ),
        ])));
    }

    rows
}
