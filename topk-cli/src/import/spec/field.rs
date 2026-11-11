#![allow(deprecated)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize, Serializer};
use topk_rs::proto::v1::control::{
    field_index, field_type::DataType, field_type_list::ListValueType,
    field_type_matrix::MatrixValueType, FieldIndex, FieldSpec, KeywordIndexType,
    MultiVectorDistanceMetric, MultiVectorIndex, MultiVectorQuantization, SemanticIndex,
    VectorDistanceMetric,
};

use crate::import::Error;

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(try_from = "FieldInput")]
pub struct Field {
    pub from: Option<String>,
    pub truncate: Option<usize>,
    pub schema: FieldSpec,
}

impl Field {
    pub fn new(schema: FieldSpec) -> Self {
        Self {
            from: None,
            truncate: None,
            schema,
        }
    }

    pub fn source<'a>(&'a self, name: &'a str) -> &'a str {
        self.from.as_deref().unwrap_or(name)
    }

    pub fn validate(&self) -> Result<(), Error> {
        validate_field(&self.schema, self.truncate)
    }
}

impl Serialize for Field {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.validate().map_err(serde::ser::Error::custom)?;
        let input = FieldInput::try_from(self).map_err(serde::ser::Error::custom)?;
        input.serialize(serializer)
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FieldInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    from: Option<String>,
    #[serde(rename = "type")]
    ty: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    truncate: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dim: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cols: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fields: Option<BTreeMap<String, Field>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    index: Option<IndexInput>,
}

impl TryFrom<FieldInput> for Field {
    type Error = Error;

    fn try_from(input: FieldInput) -> Result<Self, Error> {
        let vector = input.ty.ends_with("_vector") && !input.ty.ends_with("_sparse_vector");
        let matrix = input.ty.ends_with("_matrix");
        for (name, present, allowed, required) in [
            ("dim", input.dim.is_some(), vector, true),
            ("cols", input.cols.is_some(), matrix, true),
            (
                "fields",
                input.fields.is_some(),
                input.ty == "struct",
                false,
            ),
        ] {
            if present && !allowed {
                return Err(Error::InvalidArgument(format!(
                    "unknown field `{name}` for {}",
                    input.ty
                )));
            }
            if required && allowed && !present {
                return Err(Error::InvalidArgument(format!(
                    "missing field `{name}` for {}",
                    input.ty
                )));
            }
        }
        if input
            .fields
            .iter()
            .flatten()
            .any(|(_, field)| field.from.is_some() || field.truncate.is_some())
        {
            return Err(Error::InvalidArgument(
                "nested schema fields cannot rename or truncate source values".into(),
            ));
        }
        let mut schema = match input.ty.as_str() {
            "text" => FieldSpec::text(input.required),
            "int" => FieldSpec::integer(input.required),
            "float" => FieldSpec::float(input.required),
            "bool" => FieldSpec::boolean(input.required),
            "bytes" => FieldSpec::bytes(input.required),
            "image" => FieldSpec::image(input.required),
            "timestamp" => FieldSpec::timestamp(input.required),
            "struct" => FieldSpec::r#struct(
                input.required,
                input
                    .fields
                    .into_iter()
                    .flatten()
                    .map(|(name, field)| (name, field.schema)),
            ),
            "text_list" => FieldSpec::list(input.required, ListValueType::String),
            "int_list" => FieldSpec::list(input.required, ListValueType::Integer),
            "float_list" => FieldSpec::list(input.required, ListValueType::Float),
            "f32_vector" => FieldSpec::f32_vector(input.dim.unwrap_or_default(), input.required),
            "f16_vector" => FieldSpec::f16_vector(input.dim.unwrap_or_default(), input.required),
            "f8_vector" => FieldSpec::f8_vector(input.dim.unwrap_or_default(), input.required),
            "u8_vector" => FieldSpec::u8_vector(input.dim.unwrap_or_default(), input.required),
            "i8_vector" => FieldSpec::i8_vector(input.dim.unwrap_or_default(), input.required),
            "binary_vector" => {
                FieldSpec::binary_vector(input.dim.unwrap_or_default(), input.required)
            }
            "f32_sparse_vector" => FieldSpec::f32_sparse_vector(input.required),
            "f32_matrix" => FieldSpec::matrix(
                input.required,
                input.cols.unwrap_or_default(),
                MatrixValueType::F32,
            ),
            "f16_sparse_vector" => FieldSpec::f16_sparse_vector(input.required),
            "f16_matrix" => FieldSpec::matrix(
                input.required,
                input.cols.unwrap_or_default(),
                MatrixValueType::F16,
            ),
            "f8_sparse_vector" => FieldSpec::f8_sparse_vector(input.required),
            "f8_matrix" => FieldSpec::matrix(
                input.required,
                input.cols.unwrap_or_default(),
                MatrixValueType::F8,
            ),
            "u8_sparse_vector" => FieldSpec::u8_sparse_vector(input.required),
            "u8_matrix" => FieldSpec::matrix(
                input.required,
                input.cols.unwrap_or_default(),
                MatrixValueType::U8,
            ),
            "i8_sparse_vector" => FieldSpec::i8_sparse_vector(input.required),
            "i8_matrix" => FieldSpec::matrix(
                input.required,
                input.cols.unwrap_or_default(),
                MatrixValueType::I8,
            ),
            _ => {
                return Err(Error::InvalidArgument(format!(
                    "unknown field type `{}`",
                    input.ty
                )))
            }
        };
        schema.index = input.index.map(FieldIndex::try_from).transpose()?;
        let field = Self {
            from: input.from,
            truncate: input.truncate,
            schema,
        };
        field.validate()?;
        Ok(field)
    }
}

impl TryFrom<&Field> for FieldInput {
    type Error = Error;

    fn try_from(field: &Field) -> Result<Self, Error> {
        let mut dim = None;
        let mut cols = None;
        let mut fields = None;
        let ty = field
            .schema
            .data_type
            .as_ref()
            .and_then(|ty| ty.data_type.as_ref())
            .ok_or_else(|| Error::InvalidArgument("source field has no data type".into()))?;
        let ty = match ty {
            DataType::Text(_) => "text".into(),
            DataType::Integer(_) => "int".into(),
            DataType::Float(_) => "float".into(),
            DataType::Boolean(_) => "bool".into(),
            DataType::Bytes(_) => "bytes".into(),
            DataType::Timestamp(_) => "timestamp".into(),
            DataType::F32Vector(v) => {
                dim = Some(v.dimension);
                "f32_vector".into()
            }
            DataType::F16Vector(v) => {
                dim = Some(v.dimension);
                "f16_vector".into()
            }
            DataType::F8Vector(v) => {
                dim = Some(v.dimension);
                "f8_vector".into()
            }
            DataType::U8Vector(v) => {
                dim = Some(v.dimension);
                "u8_vector".into()
            }
            DataType::I8Vector(v) => {
                dim = Some(v.dimension);
                "i8_vector".into()
            }
            DataType::BinaryVector(v) => {
                dim = Some(v.dimension);
                "binary_vector".into()
            }
            DataType::F32SparseVector(_) => "f32_sparse_vector".into(),
            DataType::F16SparseVector(_) => "f16_sparse_vector".into(),
            DataType::F8SparseVector(_) => "f8_sparse_vector".into(),
            DataType::U8SparseVector(_) => "u8_sparse_vector".into(),
            DataType::I8SparseVector(_) => "i8_sparse_vector".into(),
            DataType::Matrix(m) => {
                cols = Some(m.dimension);
                match m.value_type() {
                    MatrixValueType::F32 => "f32_matrix",
                    MatrixValueType::F16 => "f16_matrix",
                    MatrixValueType::F8 => "f8_matrix",
                    MatrixValueType::U8 => "u8_matrix",
                    MatrixValueType::I8 => "i8_matrix",
                    _ => {
                        return Err(Error::InvalidArgument(
                            "unsupported matrix element type".into(),
                        ))
                    }
                }
                .into()
            }
            DataType::List(list) => match list.value_type() {
                ListValueType::String => "text_list",
                ListValueType::Integer => "int_list",
                ListValueType::Float => "float_list",
                _ => {
                    return Err(Error::InvalidArgument(
                        "unsupported list element type".into(),
                    ))
                }
            }
            .into(),
            DataType::Struct(nested) => {
                if !nested.fields.is_empty() {
                    fields = Some(
                        nested
                            .fields
                            .iter()
                            .map(|(name, schema)| (name.clone(), Field::new(schema.clone())))
                            .collect(),
                    );
                }
                "struct".into()
            }
            DataType::Image(_) => "image".into(),
        };
        Ok(Self {
            ty,
            dim,
            cols,
            fields,
            from: field.from.clone(),
            truncate: field.truncate,
            required: field.schema.required,
            index: field
                .schema
                .index
                .as_ref()
                .map(IndexInput::try_from)
                .transpose()?,
        })
    }
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
enum IndexInput {
    Name(String),
    Options(IndexOptions),
}

impl TryFrom<IndexInput> for FieldIndex {
    type Error = Error;

    fn try_from(input: IndexInput) -> Result<Self, Error> {
        Ok(match input {
            IndexInput::Name(name) if name == "keyword" => {
                FieldIndex::keyword(KeywordIndexType::Text)
            }
            IndexInput::Name(name) if name == "exact" => {
                FieldIndex::keyword(KeywordIndexType::Exact)
            }
            IndexInput::Name(name) if name == "semantic" => FieldIndex::semantic(),
            IndexInput::Name(name) if name == "ngram" => FieldIndex::ngram(),
            IndexInput::Options(IndexOptions::Vector { metric, exact }) => FieldIndex {
                index: Some(field_index::Index::VectorIndex(
                    topk_rs::proto::v1::control::VectorIndex {
                        metric: metric as i32,
                        exact,
                    },
                )),
            },
            IndexInput::Options(IndexOptions::MultiVector {
                quantization,
                skip_smve,
                width,
                top_k,
                encoding_version,
                sketch_bits,
            }) => FieldIndex {
                index: Some(field_index::Index::MultiVectorIndex(MultiVectorIndex {
                    metric: MultiVectorDistanceMetric::Maxsim as i32,
                    quantization: quantization.map(|q| q as i32),
                    skip_smve,
                    width,
                    top_k,
                    encoding_version,
                    sketch_bits,
                })),
            },
            IndexInput::Name(name) => {
                return Err(Error::InvalidArgument(format!("unknown index `{name}`")))
            }
            IndexInput::Options(IndexOptions::Semantic(index)) => FieldIndex {
                index: Some(field_index::Index::SemanticIndex(index)),
            },
        })
    }
}

impl TryFrom<&FieldIndex> for IndexInput {
    type Error = Error;

    fn try_from(index: &FieldIndex) -> Result<Self, Error> {
        let invalid = || Error::InvalidArgument("unsupported source index".into());
        Ok(match index.index.as_ref().ok_or_else(invalid)? {
            field_index::Index::KeywordIndex(index) => Self::Name(match index.index_type() {
                KeywordIndexType::Text => "keyword".into(),
                KeywordIndexType::Exact => "exact".into(),
                _ => return Err(invalid()),
            }),
            field_index::Index::NgramIndex(_) => Self::Name("ngram".into()),
            field_index::Index::SemanticIndex(index) if *index == SemanticIndex::default() => {
                Self::Name("semantic".into())
            }
            field_index::Index::SemanticIndex(index) => {
                Self::Options(IndexOptions::Semantic(index.clone()))
            }
            field_index::Index::VectorIndex(index) => Self::Options(IndexOptions::Vector {
                metric: match index.metric() {
                    VectorDistanceMetric::Cosine => Metric::Cosine,
                    VectorDistanceMetric::Euclidean => Metric::Euclidean,
                    VectorDistanceMetric::DotProduct => Metric::DotProduct,
                    VectorDistanceMetric::Hamming => Metric::Hamming,
                    _ => return Err(invalid()),
                },
                exact: index.exact,
            }),
            field_index::Index::MultiVectorIndex(index) => {
                Self::Options(IndexOptions::MultiVector {
                    quantization: index
                        .quantization
                        .map(|q| match MultiVectorQuantization::try_from(q) {
                            Ok(MultiVectorQuantization::Unspecified) => Ok(Quant::Unspecified),
                            Ok(MultiVectorQuantization::Binary1bit) => Ok(Quant::Binary1bit),
                            Ok(MultiVectorQuantization::Binary2bit) => Ok(Quant::Binary2bit),
                            Ok(MultiVectorQuantization::Scalar) => Ok(Quant::Scalar),
                            _ => Err(invalid()),
                        })
                        .transpose()?,
                    skip_smve: index.skip_smve,
                    width: index.width,
                    top_k: index.top_k,
                    encoding_version: index.encoding_version,
                    sketch_bits: index.sketch_bits,
                })
            }
        })
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum IndexOptions {
    Vector {
        metric: Metric,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exact: Option<bool>,
    },
    MultiVector {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        quantization: Option<Quant>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        skip_smve: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        top_k: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sketch_bits: Option<u32>,
        #[serde(default, skip_serializing_if = "is_zero")]
        encoding_version: u32,
    },
    Semantic(#[serde(with = "SemanticInput")] SemanticIndex),
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Metric {
    Cosine = VectorDistanceMetric::Cosine as isize,
    Euclidean = VectorDistanceMetric::Euclidean as isize,
    DotProduct = VectorDistanceMetric::DotProduct as isize,
    Hamming = VectorDistanceMetric::Hamming as isize,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
enum Quant {
    #[serde(rename = "unspecified")]
    Unspecified = MultiVectorQuantization::Unspecified as isize,
    #[serde(rename = "1bit")]
    Binary1bit = MultiVectorQuantization::Binary1bit as isize,
    #[serde(rename = "2bit")]
    Binary2bit = MultiVectorQuantization::Binary2bit as isize,
    #[serde(rename = "scalar")]
    Scalar = MultiVectorQuantization::Scalar as isize,
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}

#[derive(Deserialize, Serialize)]
#[serde(remote = "SemanticIndex", deny_unknown_fields)]
struct SemanticInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    embedding_type: Option<i32>,
}

fn validate_field(schema: &FieldSpec, truncate: Option<usize>) -> Result<(), Error> {
    let ty = schema
        .data_type
        .as_ref()
        .and_then(|ty| ty.data_type.as_ref())
        .ok_or_else(|| Error::InvalidArgument("source field has no data type".into()))?;
    match ty {
        DataType::List(list)
            if !matches!(
                ListValueType::try_from(list.value_type),
                Ok(ListValueType::String | ListValueType::Integer | ListValueType::Float)
            ) =>
        {
            return Err(Error::InvalidArgument(
                "unsupported list element type".into(),
            ))
        }
        DataType::Matrix(matrix)
            if !matches!(
                MatrixValueType::try_from(matrix.value_type),
                Ok(MatrixValueType::F32
                    | MatrixValueType::F16
                    | MatrixValueType::F8
                    | MatrixValueType::U8
                    | MatrixValueType::I8)
            ) =>
        {
            return Err(Error::InvalidArgument(
                "unsupported matrix element type".into(),
            ))
        }
        DataType::Struct(nested) => {
            for schema in nested.fields.values() {
                validate_field(schema, None)?;
            }
        }
        _ => {}
    }
    if truncate.is_some() && !matches!(ty, DataType::Text(_)) {
        return Err(Error::InvalidArgument(
            "unknown field `truncate` for non-text field".into(),
        ));
    }
    if let Some(index) = &schema.index {
        let valid = match &index.index {
            Some(field_index::Index::KeywordIndex(index)) => {
                matches!(
                    KeywordIndexType::try_from(index.index_type),
                    Ok(KeywordIndexType::Text | KeywordIndexType::Exact)
                ) && matches!(ty, DataType::Text(_))
            }
            Some(field_index::Index::SemanticIndex(_)) => {
                matches!(ty, DataType::Text(_) | DataType::Image(_))
            }
            Some(field_index::Index::NgramIndex(_)) => {
                matches!(ty, DataType::Text(_))
            }
            Some(field_index::Index::VectorIndex(index)) => {
                matches!(
                    VectorDistanceMetric::try_from(index.metric),
                    Ok(VectorDistanceMetric::Cosine
                        | VectorDistanceMetric::Euclidean
                        | VectorDistanceMetric::DotProduct
                        | VectorDistanceMetric::Hamming)
                ) && matches!(
                    ty,
                    DataType::F32Vector(_)
                        | DataType::F16Vector(_)
                        | DataType::F8Vector(_)
                        | DataType::U8Vector(_)
                        | DataType::I8Vector(_)
                        | DataType::BinaryVector(_)
                        | DataType::F32SparseVector(_)
                        | DataType::F16SparseVector(_)
                        | DataType::F8SparseVector(_)
                        | DataType::U8SparseVector(_)
                        | DataType::I8SparseVector(_)
                )
            }
            Some(field_index::Index::MultiVectorIndex(index)) => {
                index.metric == MultiVectorDistanceMetric::Maxsim as i32
                    && index
                        .quantization
                        .is_none_or(|q| MultiVectorQuantization::try_from(q).is_ok())
                    && matches!(ty, DataType::Matrix(_))
            }
            None => false,
        };
        if !valid {
            return Err(Error::InvalidArgument(
                "incompatible index for field type".into(),
            ));
        }
    }
    Ok(())
}
