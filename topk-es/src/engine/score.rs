use topk_rs::proto::v1::control::{
    field_type, field_type_matrix::MatrixValueType, FieldSpec, MultiVectorDistanceMetric,
    VectorDistanceMetric,
};
use topk_rs::proto::v1::data::{FunctionExpr, LogicalExpr, Query as TopkQuery, TextExpr, Value};
use topk_rs::query::{field, fns};

use super::field::IndexKind;
use super::value::ValueExt;
use super::{Schema, RANK_ANN};
use crate::api::QueryVector;
use crate::Error;

pub struct CompiledQuery {
    pub gate: LogicalExpr,
    pub score: Score,
}

#[derive(Clone, Default)]
pub struct Score {
    pub bm25: Option<TextExpr>,
    pub anns: Vec<AnnTerm>,
    pub expr: Option<LogicalExpr>,
}

impl Score {
    // The weighted sum of the parts; BM25 scales through its text term weights.
    pub fn sum(parts: Vec<Score>, factor: f32) -> Score {
        let mut sum = parts.into_iter().fold(Score::default(), |mut acc, part| {
            acc.bm25 = match (acc.bm25, part.bm25) {
                (Some(a), Some(b)) => Some(a.or(b)),
                (a, b) => a.or(b),
            };
            acc.expr = match (acc.expr, part.expr) {
                (Some(a), Some(b)) => Some(a.add(b)),
                (a, b) => a.or(b),
            };
            acc.anns.extend(part.anns);
            acc
        });

        sum.bm25 = sum.bm25.map(|text| text.boost(factor));
        sum.expr = sum.expr.map(|e| e * factor);
        for ann in &mut sum.anns {
            ann.weight *= factor;
        }
        sum
    }

    pub fn constant(boost: Option<f32>) -> Score {
        Score {
            expr: Some(LogicalExpr::literal(boost.unwrap_or(1.0))),
            ..Score::default()
        }
    }
}

#[derive(Clone)]
pub struct AnnTerm {
    pub field: String,
    pub weight: f32,
    pub query: AnnQuery,
}

#[derive(Clone)]
pub enum AnnQuery {
    Semantic(String),

    Vector {
        vector: QueryVector,
        num_candidates: Option<u64>,
    },
}

// Folds a raw vector score into ES's [0, 1] `_score` space: cosine/dot_product
// through (1 + s) / 2 (so orthogonal unit vectors report 0.5, not 0), distance
// metrics through 1 / (1 + d), maxsim unchanged. `None` => not knn-searchable.
fn normalize_score(kind: IndexKind, raw: LogicalExpr) -> Option<LogicalExpr> {
    match kind {
        IndexKind::Vector(VectorDistanceMetric::Cosine | VectorDistanceMetric::DotProduct) => Some(
            LogicalExpr::literal(1.0f32)
                .add(raw)
                .div(LogicalExpr::literal(2.0f32)),
        ),
        IndexKind::Vector(VectorDistanceMetric::Euclidean | VectorDistanceMetric::Hamming) => {
            Some(LogicalExpr::literal(1.0f32).div(LogicalExpr::literal(1.0f32).add(raw)))
        }
        IndexKind::MultiVector(MultiVectorDistanceMetric::Maxsim) => Some(raw),
        _ => None,
    }
}

// Selects each ANN term's raw scorer, folds it into ES score space, and returns
// the weighted sum of the folds (the vector contribution to `RANK_SCORE`).
pub fn ann_score(
    mut query: TopkQuery,
    schema: &Schema,
    anns: &[AnnTerm],
) -> Result<(TopkQuery, Option<LogicalExpr>), Error> {
    let mut total: Option<LogicalExpr> = None;
    for (index, ann) in anns.iter().enumerate() {
        let spec = schema.get(&ann.field).ok_or_else(|| {
            Error::InvalidQuery(format!(
                "\"{}\" is not in the collection's schema",
                ann.field
            ))
        })?;

        let (spec, scorer) = match &ann.query {
            AnnQuery::Semantic(text) => {
                let spec = match IndexKind::from(spec) {
                    IndexKind::Semantic => {
                        let embedding_field = format!("_embedding.{}", ann.field);
                        schema.get(&embedding_field).ok_or_else(|| {
                            Error::InvalidQuery(format!(
                                "\"{embedding_field}\" is not in the collection's schema"
                            ))
                        })?
                    }
                    _ => spec,
                };
                (spec, fns::semantic_similarity(&ann.field, text))
            }
            AnnQuery::Vector {
                vector,
                num_candidates,
            } => (
                spec,
                knn_distance(&ann.field, vector, *num_candidates, spec)?,
            ),
        };

        let rank_ann = format!("{RANK_ANN}_{index}");
        query = query.select([(rank_ann.as_str(), scorer)]);
        let folded = normalize_score(IndexKind::from(spec), field(rank_ann.as_str()))
            .ok_or_else(|| not_knn_searchable(&ann.field))?;

        let part = folded * ann.weight;
        total = Some(match total {
            Some(total) => total.add(part),
            None => part,
        });
    }

    Ok((query, total))
}

fn not_knn_searchable(field: &str) -> Error {
    Error::InvalidQuery(format!("\"{field}\" is not a knn-searchable vector field"))
}

fn knn_distance(
    field_name: &str,
    query_vector: &QueryVector,
    num_candidates: Option<u64>,
    spec: &FieldSpec,
) -> Result<FunctionExpr, Error> {
    match IndexKind::from(spec) {
        IndexKind::Vector(_) => match query_vector {
            QueryVector::Flat(v) => Ok(fns::vector_distance(
                field_name,
                query_value(field_name, spec, v)?,
            )),
            QueryVector::Matrix(_) => Err(Error::InvalidQuery(format!(
                "\"{field_name}\" is a dense_vector field; query_vector must be a flat array"
            ))),
        },
        IndexKind::MultiVector(_) => match query_vector {
            QueryVector::Matrix(v) => Ok(fns::multi_vector_distance(
                field_name,
                query_value(field_name, spec, v)?,
                num_candidates.map(|c| c as u32),
            )),
            QueryVector::Flat(_) => Err(Error::InvalidQuery(format!(
                "\"{field_name}\" is a rank_vectors field; query_vector must be an array of vectors"
            ))),
        },
        _ => Err(not_knn_searchable(field_name)),
    }
}

fn query_value(field_name: &str, spec: &FieldSpec, value: &Value) -> Result<Value, Error> {
    match spec.data_type.as_ref().and_then(|t| t.data_type.as_ref()) {
        Some(field_type::DataType::I8Vector(_)) => value.to_i8_list(),
        Some(field_type::DataType::U8Vector(_) | field_type::DataType::BinaryVector(_)) => {
            value.to_unsigned_bytes()
        }
        Some(field_type::DataType::Matrix(m)) if matches!(m.value_type(), MatrixValueType::U8) => {
            value.to_u8_matrix()
        }
        _ => value.to_f32_list().or_else(|| Some(value.clone())),
    }
    .ok_or_else(|| {
        Error::InvalidQuery(format!(
            "\"query_vector\" is not compatible with the type of field \"{field_name}\""
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // The exact numeric fold ((1 + s) / 2 etc.) is evaluated by the executor, so
    // its value is covered by integration tests. Here we lock the pure decision:
    // metric → fold family, and the not-knn-searchable gate (`None`).
    #[test]
    fn fold_family_per_metric() {
        let raw = || field("raw");
        let affine = |x: LogicalExpr| {
            LogicalExpr::literal(1.0f32)
                .add(x)
                .div(LogicalExpr::literal(2.0f32))
        };
        let inverse =
            |x: LogicalExpr| LogicalExpr::literal(1.0f32).div(LogicalExpr::literal(1.0f32).add(x));

        for metric in [
            VectorDistanceMetric::Cosine,
            VectorDistanceMetric::DotProduct,
        ] {
            assert_eq!(
                normalize_score(IndexKind::Vector(metric), raw()),
                Some(affine(raw()))
            );
        }
        for metric in [
            VectorDistanceMetric::Euclidean,
            VectorDistanceMetric::Hamming,
        ] {
            assert_eq!(
                normalize_score(IndexKind::Vector(metric), raw()),
                Some(inverse(raw()))
            );
        }
        // Maxsim is passed through unchanged.
        assert_eq!(
            normalize_score(
                IndexKind::MultiVector(MultiVectorDistanceMetric::Maxsim),
                raw()
            ),
            Some(raw())
        );
    }

    #[test]
    fn non_knn_kinds_are_not_searchable() {
        for kind in [
            IndexKind::Vector(VectorDistanceMetric::Unspecified),
            IndexKind::Keyword(topk_rs::proto::v1::control::KeywordIndexType::Exact),
            IndexKind::Semantic,
            IndexKind::None,
        ] {
            assert!(normalize_score(kind, field("raw")).is_none());
        }
    }
}
