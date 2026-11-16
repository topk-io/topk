use std::str::FromStr;

use topk_rs::proto::v1::control::{
    FieldIndex, FieldSpec, KeywordIndexType, MultiVectorDistanceMetric, VectorDistanceMetric,
};

use crate::import::plan::{Field, FieldSelection, Rename, Type};
use crate::import::Error;

impl FromStr for FieldSelection {
    type Err = Error;

    fn from_str(raw: &str) -> Result<Self, Error> {
        let (name, value) = match raw.split_once('=') {
            Some((name, value)) => (name, Some(value)),
            None => (raw, None),
        };
        if name.is_empty() || name.starts_with('_') {
            return Err(Error::InvalidArgument(format!(
                "invalid field name {name:?}"
            )));
        }
        let (ty, index) = match value {
            None => {
                return Err(Error::InvalidArgument(format!(
                    "-f takes <field>=<type or index>, e.g. {name}=keyword; to leave {name} out, use --exclude {name}"
                )))
            }
            Some(value) => {
                let (kind, suffix) = value
                    .split_once(':')
                    .map_or((value, None), |(kind, suffix)| (kind, Some(suffix)));
                match kind {
                    "vector" => {
                        let (dim, metric) = parse_shape(value, kind)?;
                        (
                            Some(Type::Vector(dim)),
                            metric
                                .map(parse_metric)
                                .transpose()?
                                .map(FieldIndex::vector),
                        )
                    }
                    "matrix" => {
                        let (cols, index) = parse_shape(value, kind)?;
                        if !matches!(index, None | Some("maxsim")) {
                            return Err(Error::InvalidArgument(format!(
                                "unknown matrix index {index:?}"
                            )));
                        }
                        (
                            Some(Type::Matrix(cols)),
                            index.map(|_| {
                                FieldIndex::multi_vector(
                                    MultiVectorDistanceMetric::Maxsim,
                                    None,
                                    None,
                                    None,
                                )
                            }),
                        )
                    }
                    "sparse" if matches!(suffix, None | Some("dot_product")) => (
                        Some(Type::Sparse),
                        suffix.map(|_| FieldIndex::vector(VectorDistanceMetric::DotProduct)),
                    ),
                    "keyword" | "semantic" | "ngram" if suffix.is_none() => {
                        (None, Some(parse_index(kind)?))
                    }
                    _ => {
                        let ty = match kind {
                            "text" => Type::Text,
                            "int" => Type::Int,
                            "float" => Type::Float,
                            "bool" => Type::Bool,
                            "bytes" => Type::Bytes,
                            "image" => Type::Image,
                            "timestamp" => Type::Timestamp,
                            "text_list" => Type::TextList,
                            "int_list" => Type::IntList,
                            "float_list" => Type::FloatList,
                            _ => {
                                return Err(Error::InvalidArgument(format!(
                                    "unknown field choice {value:?}"
                                )))
                            }
                        };
                        (Some(ty), suffix.map(parse_index).transpose()?)
                    }
                }
            }
        };
        let selection = Self {
            name: name.into(),
            ty,
            index,
        };
        if let Some(ty) = selection.ty.as_ref().and_then(Type::field_type) {
            Field::new(FieldSpec {
                data_type: Some(ty),
                index: selection.index.clone(),
                required: false,
            })
            .validate()?;
        }
        Ok(selection)
    }
}

fn parse_shape<'a>(value: &'a str, prefix: &str) -> Result<(Option<u32>, Option<&'a str>), Error> {
    let suffix = value.strip_prefix(prefix).unwrap_or_default();
    if suffix.is_empty() {
        return Ok((None, None));
    }
    let suffix = suffix
        .strip_prefix(':')
        .ok_or_else(|| Error::InvalidArgument(format!("unknown field choice {value:?}")))?;
    let (first, second) = match suffix.split_once(':') {
        Some((first, second)) => (first, Some(second)),
        None => (suffix, None),
    };
    if let Ok(dim) = first.parse::<u32>() {
        if dim == 0 || second == Some("") {
            return Err(Error::InvalidArgument(format!(
                "invalid field choice {value:?}"
            )));
        }
        return Ok((Some(dim), second));
    }
    if second.is_some() || first.is_empty() {
        return Err(Error::InvalidArgument(format!(
            "invalid field choice {value:?}"
        )));
    }
    Ok((None, Some(first)))
}

fn parse_index(index: &str) -> Result<FieldIndex, Error> {
    match index {
        "keyword" => Ok(FieldIndex::keyword(KeywordIndexType::Text)),
        "semantic" => Ok(FieldIndex::semantic()),
        "ngram" => Ok(FieldIndex::ngram()),
        _ => Err(Error::InvalidArgument(format!(
            "unknown field index {index:?}"
        ))),
    }
}

fn parse_metric(metric: &str) -> Result<VectorDistanceMetric, Error> {
    match metric {
        "cosine" => Ok(VectorDistanceMetric::Cosine),
        "euclidean" => Ok(VectorDistanceMetric::Euclidean),
        "dot_product" => Ok(VectorDistanceMetric::DotProduct),
        "hamming" => Ok(VectorDistanceMetric::Hamming),
        _ => Err(Error::InvalidArgument(format!(
            "unknown vector metric {metric:?}"
        ))),
    }
}

impl FromStr for Rename {
    type Err = Error;

    fn from_str(raw: &str) -> Result<Self, Error> {
        let (source, target) = raw
            .split_once('=')
            .ok_or_else(|| Error::InvalidArgument("rename must be source=target".into()))?;
        if source.is_empty() || target.is_empty() || target.starts_with('_') || target.contains('=')
        {
            return Err(Error::InvalidArgument(format!("invalid rename {raw:?}")));
        }
        Ok(Self {
            source: source.into(),
            target: target.into(),
        })
    }
}
