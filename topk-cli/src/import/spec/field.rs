use serde::{Deserialize, Serialize};
use topk_rs::proto::v1::control::field_type_list::ListValueType;
use topk_rs::proto::v1::control::field_type_matrix::MatrixValueType;
use topk_rs::proto::v1::control::{
    field_index, field_type, FieldIndex, FieldSpec, KeywordIndexType, MultiVectorDistanceMetric,
    MultiVectorQuantization, VectorDistanceMetric,
};

use crate::import::error::Error;

#[derive(Clone, Default, Deserialize, PartialEq, Serialize)]
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

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, strum::Display)]
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
    pub fn is_dense(self) -> bool {
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

    pub fn is_scalar(self) -> bool {
        matches!(
            self,
            Type::Text | Type::Int | Type::Float | Type::Bool | Type::Bytes | Type::Timestamp
        )
    }

    pub fn is_matrix(self) -> bool {
        matches!(
            self,
            Type::F32Matrix | Type::F16Matrix | Type::F8Matrix | Type::U8Matrix | Type::I8Matrix
        )
    }

    /// Element type of a vector, matrix or sparse vector; `FloatList` is f32.
    pub fn element(self) -> Option<Element> {
        Some(match self {
            Type::F32Vector | Type::F32Matrix | Type::F32SparseVector | Type::FloatList => {
                Element::F32
            }
            Type::F16Vector | Type::F16Matrix | Type::F16SparseVector => Element::F16,
            Type::F8Vector | Type::F8Matrix | Type::F8SparseVector => Element::F8,
            Type::U8Vector | Type::BinaryVector | Type::U8Matrix | Type::U8SparseVector => {
                Element::U8
            }
            Type::I8Vector | Type::I8Matrix | Type::I8SparseVector => Element::I8,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Element {
    F32,
    F16,
    F8,
    U8,
    I8,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Index {
    Keyword,
    Exact,
    Semantic,
    Ngram,
    Vector {
        metric: Metric,
    },
    MultiVector {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        quantization: Option<Quant>,

        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        skip_smve: bool,
    },
}

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    Cosine,
    Euclidean,
    DotProduct,
    Hamming,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
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
        let vector = field.ty.is_dense();
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

impl From<&FieldSpec> for Field {
    fn from(spec: &FieldSpec) -> Self {
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
                ListValueType::Integer => Type::IntList,
                ListValueType::Float => Type::FloatList,
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
                    MatrixValueType::F16 => Type::F16Matrix,
                    MatrixValueType::F8 => Type::F8Matrix,
                    MatrixValueType::U8 => Type::U8Matrix,
                    MatrixValueType::I8 => Type::I8Matrix,
                    _ => Type::F32Matrix,
                },
                cols: Some(m.dimension),
                ..Default::default()
            },
        }
    }
}

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

impl From<Index> for FieldIndex {
    fn from(index: Index) -> Self {
        match index {
            Index::Keyword => FieldIndex::keyword(KeywordIndexType::Text),
            Index::Exact => FieldIndex::keyword(KeywordIndexType::Exact),
            Index::Semantic => FieldIndex::semantic(),
            Index::Ngram => FieldIndex::ngram(),
            Index::Vector { metric } => FieldIndex::vector(metric.into()),
            Index::MultiVector {
                quantization,
                skip_smve,
            } => {
                let index = FieldIndex::multi_vector(
                    MultiVectorDistanceMetric::Maxsim,
                    quantization.map(MultiVectorQuantization::from),
                    None,
                    None,
                );
                match skip_smve {
                    true => index.skip_smve(),
                    false => index,
                }
            }
        }
    }
}
