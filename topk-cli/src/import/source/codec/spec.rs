use topk_rs::proto::v1::control::{
    field_index, field_type, field_type_list, field_type_matrix, FieldSpec, KeywordIndexType,
    MultiVectorQuantization, VectorDistanceMetric,
};

use crate::import::spec::{Field, Index, Quant, Type};

/// The declared index, which a source that stores one hands us outright — no
/// guess, unlike the sniffed sources where discovery deliberately picks none.
fn index(spec: &FieldSpec) -> Option<Index> {
    Some(match spec.index.as_ref()?.index.as_ref()? {
        field_index::Index::KeywordIndex(keyword) => {
            match KeywordIndexType::try_from(keyword.index_type) {
                Ok(KeywordIndexType::Exact) => Index::Exact,
                _ => Index::Keyword,
            }
        }
        field_index::Index::SemanticIndex(_) => Index::Semantic,
        field_index::Index::NgramIndex(_) => Index::Ngram,
        field_index::Index::VectorIndex(vector) => Index::Vector {
            metric: VectorDistanceMetric::try_from(vector.metric)
                .unwrap_or(VectorDistanceMetric::Cosine)
                .into(),
        },
        field_index::Index::MultiVectorIndex(multi) => Index::MultiVector {
            quantization: multi
                .quantization
                .and_then(|quant| MultiVectorQuantization::try_from(quant).ok())
                .and_then(|quant| Quant::try_from(quant).ok()),
            skip_smve: multi.skip_smve,
        },
    })
}

pub fn field(spec: &FieldSpec) -> Field {
    let Some(data_type) = spec.data_type.as_ref().and_then(|t| t.data_type.as_ref()) else {
        return Field::default();
    };
    let index = index(spec);
    let required = spec.required;
    let plain = |ty| Field {
        ty,
        index,
        required,
        ..Default::default()
    };
    let vector = |ty, dim| Field {
        ty,
        dim: Some(dim),
        index,
        required,
        ..Default::default()
    };
    match data_type {
        field_type::DataType::Text(_) => plain(Type::Text),
        field_type::DataType::Timestamp(_) => plain(Type::Timestamp),
        field_type::DataType::Integer(_) => plain(Type::Int),
        field_type::DataType::Float(_) => plain(Type::Float),
        field_type::DataType::Boolean(_) => plain(Type::Bool),
        field_type::DataType::Bytes(_) => plain(Type::Bytes),
        field_type::DataType::Struct(_) => plain(Type::Struct),
        field_type::DataType::List(list) => plain(match list.value_type() {
            field_type_list::ListValueType::Integer => Type::IntList,
            field_type_list::ListValueType::Float => Type::FloatList,
            _ => Type::TextList,
        }),
        field_type::DataType::F32Vector(v) => vector(Type::F32Vector, v.dimension),
        field_type::DataType::F16Vector(v) => vector(Type::F16Vector, v.dimension),
        field_type::DataType::F8Vector(v) => vector(Type::F8Vector, v.dimension),
        field_type::DataType::U8Vector(v) => vector(Type::U8Vector, v.dimension),
        field_type::DataType::I8Vector(v) => vector(Type::I8Vector, v.dimension),
        field_type::DataType::BinaryVector(v) => vector(Type::BinaryVector, v.dimension),
        field_type::DataType::F32SparseVector(_) => plain(Type::F32SparseVector),
        field_type::DataType::F16SparseVector(_) => plain(Type::F16SparseVector),
        field_type::DataType::F8SparseVector(_) => plain(Type::F8SparseVector),
        field_type::DataType::U8SparseVector(_) => plain(Type::U8SparseVector),
        field_type::DataType::I8SparseVector(_) => plain(Type::I8SparseVector),
        field_type::DataType::Matrix(m) => Field {
            index,
            required,
            ty: match m.value_type() {
                field_type_matrix::MatrixValueType::F16 => Type::F16Matrix,
                field_type_matrix::MatrixValueType::F8 => Type::F8Matrix,
                field_type_matrix::MatrixValueType::U8 => Type::U8Matrix,
                field_type_matrix::MatrixValueType::I8 => Type::I8Matrix,
                _ => Type::F32Matrix,
            },
            cols: Some(m.dimension),
            ..Default::default()
        },
    }
}
