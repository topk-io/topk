use crate::control;
use pyo3::prelude::*;

////////////////////////////////////////////////////////////
/// Schema
///
/// This module contains the schema definition for the TopK SDK.
////////////////////////////////////////////////////////////

#[pymodule]
#[pyo3(name = "schema")]
pub fn pymodule(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // data types
    m.add_wrapped(wrap_pyfunction!(text))?;
    m.add_wrapped(wrap_pyfunction!(vector))?;
    m.add_wrapped(wrap_pyfunction!(int))?;

    // indexes
    m.add_wrapped(wrap_pyfunction!(vector_index))?;
    m.add_wrapped(wrap_pyfunction!(keyword_index))?;

    Ok(())
}

#[pyfunction]
pub fn text() -> control::field_spec::FieldSpec {
    control::field_spec::FieldSpec::new(control::data_type::DataType::Text())
}

#[pyfunction]
pub fn vector(dimension: u32) -> control::field_spec::FieldSpec {
    // TODO: why u32?
    control::field_spec::FieldSpec::new(control::data_type::DataType::FloatVector { dimension })
}

#[pyfunction]
pub fn int() -> control::field_spec::FieldSpec {
    control::field_spec::FieldSpec::new(control::data_type::DataType::Integer())
}

#[pyfunction]
pub fn vector_index(metric: String) -> PyResult<control::field_index::FieldIndex> {
    let metric = match metric.to_lowercase().as_str() {
        "cosine" => control::field_index::VectorDistanceMetric::Cosine,
        "euclidean" => control::field_index::VectorDistanceMetric::Euclidean,
        "dot_product" => control::field_index::VectorDistanceMetric::DotProduct,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid vector distance metric: {}. Supported metrics are: cosine, euclidean, dot_product.",
                metric
            )))
        }
    };

    Ok(control::field_index::FieldIndex::VectorIndex { metric })
}

#[pyfunction]
#[pyo3(signature = (r#type="text".into()))]
pub fn keyword_index(r#type: String) -> PyResult<control::field_index::FieldIndex> {
    let index_type = match r#type.to_lowercase().as_str() {
        "text" => control::field_index::KeywordIndexType::Text,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid keyword index type: {}. Supported index types are: text.",
                r#type
            )))
        }
    };

    Ok(control::field_index::FieldIndex::KeywordIndex { index_type })
}
