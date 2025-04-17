use pyo3::prelude::*;

pub mod data_type;
pub mod field_index;
pub mod field_spec;

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
    m.add_wrapped(wrap_pyfunction!(int))?;
    m.add_wrapped(wrap_pyfunction!(float))?;
    m.add_wrapped(wrap_pyfunction!(self::bool))?;
    m.add_wrapped(wrap_pyfunction!(f32_vector))?;
    m.add_wrapped(wrap_pyfunction!(u8_vector))?;
    m.add_wrapped(wrap_pyfunction!(binary_vector))?;
    m.add_wrapped(wrap_pyfunction!(bytes))?;

    // indexes
    m.add_wrapped(wrap_pyfunction!(vector_index))?;
    m.add_wrapped(wrap_pyfunction!(keyword_index))?;
    m.add_wrapped(wrap_pyfunction!(semantic_index))?;

    Ok(())
}

#[pyfunction]
pub fn text() -> field_spec::FieldSpec {
    field_spec::FieldSpec::new(data_type::DataType::Text())
}

#[pyfunction]
pub fn int() -> field_spec::FieldSpec {
    field_spec::FieldSpec::new(data_type::DataType::Integer())
}

#[pyfunction]
pub fn float() -> field_spec::FieldSpec {
    field_spec::FieldSpec::new(data_type::DataType::Float())
}

#[pyfunction]
pub fn bool() -> field_spec::FieldSpec {
    field_spec::FieldSpec::new(data_type::DataType::Boolean())
}

#[pyfunction]
pub fn f32_vector(dimension: u32) -> field_spec::FieldSpec {
    field_spec::FieldSpec::new(data_type::DataType::F32Vector { dimension })
}

#[pyfunction]
pub fn u8_vector(dimension: u32) -> field_spec::FieldSpec {
    field_spec::FieldSpec::new(data_type::DataType::U8Vector { dimension })
}

#[pyfunction]
pub fn binary_vector(dimension: u32) -> field_spec::FieldSpec {
    field_spec::FieldSpec::new(data_type::DataType::BinaryVector { dimension })
}

#[pyfunction]
pub fn bytes() -> field_spec::FieldSpec {
    field_spec::FieldSpec::new(data_type::DataType::Bytes())
}

#[pyfunction]
pub fn vector_index(metric: String) -> PyResult<field_index::FieldIndex> {
    let metric = match metric.to_lowercase().as_str() {
        "cosine" => field_index::VectorDistanceMetric::Cosine,
        "euclidean" => field_index::VectorDistanceMetric::Euclidean,
        "dot_product" => field_index::VectorDistanceMetric::DotProduct,
        "hamming" => field_index::VectorDistanceMetric::Hamming,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid vector distance metric: {}. Supported metrics are: cosine, euclidean, dot_product, hamming.",
                metric
            )))
        }
    };

    Ok(field_index::FieldIndex::VectorIndex { metric })
}

#[pyfunction]
#[pyo3(signature = (r#type="text".into()))]
pub fn keyword_index(r#type: String) -> PyResult<field_index::FieldIndex> {
    let index_type = match r#type.to_lowercase().as_str() {
        "text" => field_index::KeywordIndexType::Text,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid keyword index type: {}. Supported index types are: text.",
                r#type
            )))
        }
    };

    Ok(field_index::FieldIndex::KeywordIndex { index_type })
}

#[pyfunction]
#[pyo3(signature=(model=None, embedding_type=None))]
pub fn semantic_index(
    model: Option<String>,
    embedding_type: Option<String>,
) -> PyResult<field_index::FieldIndex> {
    let embedding_type = match embedding_type {
        Some(embedding_type) => match embedding_type.as_str() {
            "f32" => Some(field_index::EmbeddingDataType::Float32),
            "u8" => Some(field_index::EmbeddingDataType::UInt8),
            "binary" => Some(field_index::EmbeddingDataType::Binary),
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid embedding type: {}",
                    embedding_type
                )))
            }
        },
        None => None,
    };

    Ok(field_index::FieldIndex::SemanticIndex {
        model,
        embedding_type: embedding_type.into(),
    })
}
