use std::collections::HashMap;

use arrow_array::Int64Array;
use arrow_array::{
    types::Float64Type, Array, LargeListArray, LargeStringArray, PrimitiveArray, RecordBatch,
};
use pyo3::types::PyDictMethods;
use pyo3::types::PyListMethods;
use pyo3::types::{PyDict, PyList};
use pyo3::IntoPyObject;
use pyo3::Py;
use pyo3::Python;
use topk_py::data::value::Value as PyValue;
use topk_rs::proto::v1::data::{Document, Value};

use crate::run_python;

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

        rows.push(Document {
            fields: HashMap::from([
                ("id".to_string(), Value::string(id)),
                ("text".to_string(), Value::string(text)),
                ("dense_embedding".to_string(), Value::list(dense_embedding)),
                ("numerical_filter".to_string(), Value::u32(numerical_filter)),
                (
                    "categorical_filter".to_string(),
                    Value::string(categorical_filter),
                ),
            ]),
        });
    }

    rows
}

pub async fn into_python(documents: Vec<Document>) -> anyhow::Result<Py<PyList>> {
    let list = run_python!(move |py| {
        let list = PyList::empty(py);
        for doc in documents {
            let dict = PyDict::new(py);

            for (key, value) in doc.fields {
                let topk_py_value = PyValue::from(value);
                let topk_py_value = topk_py_value.into_pyobject(py)?;
                dict.set_item(key, topk_py_value)?;
            }

            list.append(dict.into_pyobject(py)?)?;
        }

        Ok::<Py<PyList>, anyhow::Error>(list.into())
    })?;

    Ok(list)
}
