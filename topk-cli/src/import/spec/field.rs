use serde::{Deserialize, Serialize};
use topk_rs::proto::v1::control::field_type_list::ListValueType;
use topk_rs::proto::v1::control::field_type_matrix::MatrixValueType;
use topk_rs::proto::v1::control::{
    field_index, FieldIndex, FieldSpec, KeywordIndexType, MultiVectorDistanceMetric,
    MultiVectorIndex, MultiVectorQuantization, VectorDistanceMetric, VectorIndex,
};

use crate::import::error::Error;

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,

    #[serde(rename = "type")]
    pub ty: Type,

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub required: bool,

    /// Keep at most this many characters — declared loss, like `float` over a
    /// wide decimal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncate: Option<usize>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dim: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cols: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<Index>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Type {
    #[default]
    Text,
    Int,
    Float,
    Bool,
    Bytes,
    Timestamp,
    Struct,
    TextList,
    IntList,
    FloatList,
    F32Vector,
    F16Vector,
    F8Vector,
    U8Vector,
    I8Vector,
    BinaryVector,
    F32Matrix,
    F16Matrix,
    F8Matrix,
    U8Matrix,
    I8Matrix,
    F32SparseVector,
    F16SparseVector,
    F8SparseVector,
    U8SparseVector,
    I8SparseVector,
}

impl Type {
    pub fn is_vector(self) -> bool {
        matches!(
            self,
            Type::F32Vector
                | Type::F16Vector
                | Type::F8Vector
                | Type::U8Vector
                | Type::I8Vector
                | Type::BinaryVector
        )
    }

    pub fn is_matrix(self) -> bool {
        matches!(
            self,
            Type::F32Matrix | Type::F16Matrix | Type::F8Matrix | Type::U8Matrix | Type::I8Matrix
        )
    }

    pub fn is_sparse(self) -> bool {
        matches!(
            self,
            Type::F32SparseVector
                | Type::F16SparseVector
                | Type::F8SparseVector
                | Type::U8SparseVector
                | Type::I8SparseVector
        )
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Index {
    Keyword,
    Exact,
    Semantic,
    Ngram,
    /// `exact` is not modelled: the server reports it but refuses it on create
    /// ("setting exact on vector index is not allowed").
    Vector {
        metric: Metric,
    },
    /// `width`, `top_k` and a non-zero `encoding_version` are not modelled: the
    /// server reports them but refuses them on create.
    MultiVector {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        quantization: Option<Quant>,

        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        skip_smve: bool,
    },
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    Cosine,
    Euclidean,
    DotProduct,
    Hamming,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
pub enum Quant {
    #[serde(rename = "1bit")]
    Binary1bit,
    #[serde(rename = "2bit")]
    Binary2bit,
    #[serde(rename = "scalar")]
    Scalar,
}

impl From<Metric> for VectorDistanceMetric {
    fn from(metric: Metric) -> Self {
        match metric {
            Metric::Cosine => VectorDistanceMetric::Cosine,
            Metric::Euclidean => VectorDistanceMetric::Euclidean,
            Metric::DotProduct => VectorDistanceMetric::DotProduct,
            Metric::Hamming => VectorDistanceMetric::Hamming,
        }
    }
}

impl From<VectorDistanceMetric> for Metric {
    fn from(metric: VectorDistanceMetric) -> Self {
        match metric {
            VectorDistanceMetric::Euclidean => Metric::Euclidean,
            VectorDistanceMetric::DotProduct => Metric::DotProduct,
            VectorDistanceMetric::Hamming => Metric::Hamming,
            _ => Metric::Cosine,
        }
    }
}

impl From<Quant> for MultiVectorQuantization {
    fn from(quant: Quant) -> Self {
        match quant {
            Quant::Binary1bit => MultiVectorQuantization::Binary1bit,
            Quant::Binary2bit => MultiVectorQuantization::Binary2bit,
            Quant::Scalar => MultiVectorQuantization::Scalar,
        }
    }
}

impl TryFrom<MultiVectorQuantization> for Quant {
    type Error = ();

    fn try_from(quant: MultiVectorQuantization) -> Result<Self, ()> {
        match quant {
            MultiVectorQuantization::Binary1bit => Ok(Quant::Binary1bit),
            MultiVectorQuantization::Binary2bit => Ok(Quant::Binary2bit),
            MultiVectorQuantization::Scalar => Ok(Quant::Scalar),
            MultiVectorQuantization::Unspecified => Err(()),
        }
    }
}

impl TryFrom<&Field> for FieldSpec {
    type Error = Error;

    fn try_from(field: &Field) -> Result<Self, Self::Error> {
        let vector = field.ty.is_vector();
        let matrix = field.ty.is_matrix();

        if vector && field.dim.is_none() {
            return Err(Error::InvalidArgument(format!(
                "{} requires `dim`",
                field.ty
            )));
        }
        if !vector && field.dim.is_some() {
            return Err(Error::InvalidArgument(format!(
                "{} does not take `dim`",
                field.ty
            )));
        }
        if field.truncate.is_some() && !matches!(field.ty, Type::Text) {
            return Err(Error::InvalidArgument(format!(
                "{} does not take `truncate`",
                field.ty
            )));
        }
        if field.truncate == Some(0) {
            return Err(Error::InvalidArgument(
                "`truncate` must keep at least 1 character".to_string(),
            ));
        }
        if matrix && field.cols.is_none() {
            return Err(Error::InvalidArgument(format!(
                "{} requires `cols`",
                field.ty
            )));
        }
        if !matrix && field.cols.is_some() {
            return Err(Error::InvalidArgument(format!(
                "{} does not take `cols`",
                field.ty
            )));
        }

        if let Some(index) = field.index {
            let (ok, kind, needs) = match index {
                Index::Keyword => (matches!(field.ty, Type::Text), "keyword", "a `text` field"),
                Index::Exact => (matches!(field.ty, Type::Text), "exact", "a `text` field"),
                Index::Semantic => (matches!(field.ty, Type::Text), "semantic", "a `text` field"),
                Index::Ngram => (matches!(field.ty, Type::Text), "ngram", "a `text` field"),
                Index::Vector { .. } => (
                    vector || field.ty.is_sparse(),
                    "vector",
                    "a vector or sparse vector field",
                ),
                Index::MultiVector { .. } => (matrix, "multi_vector", "a matrix field"),
            };
            if !ok {
                return Err(Error::InvalidArgument(format!(
                    "a {kind} index needs {needs}"
                )));
            }
        }

        let spec = match field.ty {
            Type::Text => FieldSpec::text(field.required),
            Type::Int => FieldSpec::integer(field.required),
            Type::Float => FieldSpec::float(field.required),
            Type::Bool => FieldSpec::boolean(field.required),
            Type::Bytes => FieldSpec::bytes(field.required),
            Type::Timestamp => FieldSpec::timestamp(field.required),
            Type::Struct => FieldSpec::r#struct(field.required, Vec::<(String, FieldSpec)>::new()),
            Type::TextList => FieldSpec::list(field.required, ListValueType::String),
            Type::IntList => FieldSpec::list(field.required, ListValueType::Integer),
            Type::FloatList => FieldSpec::list(field.required, ListValueType::Float),
            Type::F32Vector => FieldSpec::f32_vector(field.dim.unwrap(), field.required),
            Type::F16Vector => FieldSpec::f16_vector(field.dim.unwrap(), field.required),
            Type::F8Vector => FieldSpec::f8_vector(field.dim.unwrap(), field.required),
            Type::U8Vector => FieldSpec::u8_vector(field.dim.unwrap(), field.required),
            Type::I8Vector => FieldSpec::i8_vector(field.dim.unwrap(), field.required),
            Type::BinaryVector => FieldSpec::binary_vector(field.dim.unwrap(), field.required),
            Type::F32Matrix => {
                FieldSpec::matrix(field.required, field.cols.unwrap(), MatrixValueType::F32)
            }
            Type::F16Matrix => {
                FieldSpec::matrix(field.required, field.cols.unwrap(), MatrixValueType::F16)
            }
            Type::F8Matrix => {
                FieldSpec::matrix(field.required, field.cols.unwrap(), MatrixValueType::F8)
            }
            Type::U8Matrix => {
                FieldSpec::matrix(field.required, field.cols.unwrap(), MatrixValueType::U8)
            }
            Type::I8Matrix => {
                FieldSpec::matrix(field.required, field.cols.unwrap(), MatrixValueType::I8)
            }
            Type::F32SparseVector => FieldSpec::f32_sparse_vector(field.required),
            Type::F16SparseVector => FieldSpec::f16_sparse_vector(field.required),
            Type::F8SparseVector => FieldSpec::f8_sparse_vector(field.required),
            Type::U8SparseVector => FieldSpec::u8_sparse_vector(field.required),
            Type::I8SparseVector => FieldSpec::i8_sparse_vector(field.required),
        };
        Ok(match field.index {
            Some(index) => spec.with_index(FieldIndex::from(index)),
            None => spec,
        })
    }
}

impl From<Index> for FieldIndex {
    fn from(index: Index) -> Self {
        match index {
            Index::Keyword => FieldIndex::keyword(KeywordIndexType::Text),
            Index::Exact => FieldIndex::keyword(KeywordIndexType::Exact),
            Index::Semantic => FieldIndex::semantic(),
            Index::Ngram => FieldIndex::ngram(),
            Index::Vector { metric } => FieldIndex {
                index: Some(field_index::Index::VectorIndex(VectorIndex {
                    metric: VectorDistanceMetric::from(metric).into(),
                    exact: None,
                })),
            },
            // Built here rather than with `FieldIndex::multi_vector`, which
            // fixes `skip_smve` at its default.
            Index::MultiVector {
                quantization,
                skip_smve,
            } => FieldIndex {
                index: Some(field_index::Index::MultiVectorIndex(MultiVectorIndex {
                    metric: MultiVectorDistanceMetric::Maxsim.into(),
                    #[allow(deprecated)]
                    sketch_bits: None,
                    quantization: quantization
                        .map(|quant| MultiVectorQuantization::from(quant).into()),
                    width: None,
                    top_k: None,
                    skip_smve,
                    encoding_version: 0,
                })),
            },
        }
    }
}
