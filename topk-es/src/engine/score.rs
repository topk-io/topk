use topk_rs::proto::v1::control::{
    field_type, field_type_matrix::MatrixValueType, FieldSpec, MultiVectorDistanceMetric,
    VectorDistanceMetric,
};
use topk_rs::proto::v1::data::{FunctionExpr, LogicalExpr, Query as TopkQuery, TextExpr, Value};
use topk_rs::query::{field, fns};

use super::field::IndexKind;
use super::{Schema, RANK_ANN};
use crate::api::QueryVector;
use crate::value::ValueExt;
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
    // ES `knn.similarity` cutoff, in raw metric units (a distance for
    // distance metrics) — folded into score space by `Fold::threshold`.
    pub cutoff: Option<f32>,
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

// Folds a raw vector score into ES's [0, 1] `_score` space. `expr` and
// `threshold` must stay in the same space: `expr` maps the engine's raw
// per-doc value, `threshold` maps the user's ES-units cutoff.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fold {
    // Similarity s → (1 + s) / 2, so orthogonal unit vectors report 0.5, not 0.
    Affine,
    // Distance d → 1 / (1 + d). `squared` marks metrics whose raw engine value
    // is already d² while the ES-facing cutoff is the unsquared distance
    // (TopK's euclidean kernel skips the sqrt, which matches ES's l2_norm
    // score of 1 / (1 + d²) exactly).
    Inverse { squared: bool },
    // Maxsim, unchanged (TopK extension — no ES score space to map into).
    Passthrough,
}

impl Fold {
    // `None` => not knn-searchable.
    fn of(kind: IndexKind) -> Option<Fold> {
        match kind {
            IndexKind::Vector(VectorDistanceMetric::Cosine | VectorDistanceMetric::DotProduct) => {
                Some(Fold::Affine)
            }
            IndexKind::Vector(VectorDistanceMetric::Euclidean) => {
                Some(Fold::Inverse { squared: true })
            }
            IndexKind::Vector(VectorDistanceMetric::Hamming) => {
                Some(Fold::Inverse { squared: false })
            }
            IndexKind::MultiVector(MultiVectorDistanceMetric::Maxsim) => Some(Fold::Passthrough),
            _ => None,
        }
    }

    fn expr(self, raw: LogicalExpr) -> LogicalExpr {
        match self {
            Fold::Affine => LogicalExpr::literal(1.0f32)
                .add(raw)
                .div(LogicalExpr::literal(2.0f32)),
            Fold::Inverse { .. } => {
                LogicalExpr::literal(1.0f32).div(LogicalExpr::literal(1.0f32).add(raw))
            }
            Fold::Passthrough => raw,
        }
    }

    fn threshold(self, cutoff: f32) -> f32 {
        match self {
            Fold::Affine => (1.0 + cutoff) / 2.0,
            Fold::Inverse { squared } => {
                1.0 / (1.0 + if squared { cutoff * cutoff } else { cutoff })
            }
            Fold::Passthrough => cutoff,
        }
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

        let fold = Fold::of(IndexKind::from(spec)).ok_or_else(|| match &ann.query {
            AnnQuery::Semantic(_) => Error::InvalidQuery(format!(
                "Field [{}] does not support semantic queries",
                ann.field
            )),
            AnnQuery::Vector { .. } => not_knn_searchable(&ann.field),
        })?;

        let rank_ann = format!("{RANK_ANN}_{index}");
        query = query.select([(rank_ann.as_str(), scorer)]);
        let folded = fold.expr(field(rank_ann.as_str()));

        // ES applies the `similarity` cutoff before `boost`, so compare the
        // unweighted fold.
        if let Some(cutoff) = ann.cutoff {
            query = query.filter(
                folded
                    .clone()
                    .gte(LogicalExpr::literal(fold.threshold(cutoff))),
            );
        }

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
    // metric → fold, and the not-knn-searchable gate (`None`).
    #[test]
    fn fold_per_metric() {
        for (metric, fold) in [
            (VectorDistanceMetric::Cosine, Fold::Affine),
            (VectorDistanceMetric::DotProduct, Fold::Affine),
            (
                VectorDistanceMetric::Euclidean,
                Fold::Inverse { squared: true },
            ),
            (
                VectorDistanceMetric::Hamming,
                Fold::Inverse { squared: false },
            ),
        ] {
            assert_eq!(Fold::of(IndexKind::Vector(metric)), Some(fold));
        }
        assert_eq!(
            Fold::of(IndexKind::MultiVector(MultiVectorDistanceMetric::Maxsim)),
            Some(Fold::Passthrough)
        );
    }

    // The ES `similarity` cutoff must land in the same space as the folded
    // score: for l2_norm the user passes an unsquared distance while the
    // engine's raw value (and so `expr`) works on d².
    #[test]
    fn threshold_matches_score_space() {
        assert_eq!(Fold::Affine.threshold(0.5), 0.75);
        assert_eq!(Fold::Inverse { squared: true }.threshold(5.0), 1.0 / 26.0);
        assert_eq!(Fold::Inverse { squared: false }.threshold(5.0), 1.0 / 6.0);
        assert_eq!(Fold::Passthrough.threshold(5.0), 5.0);
    }

    #[test]
    fn passthrough_expr_is_identity() {
        assert_eq!(Fold::Passthrough.expr(field("raw")), field("raw"));
    }

    #[test]
    fn non_knn_kinds_are_not_searchable() {
        for kind in [
            IndexKind::Vector(VectorDistanceMetric::Unspecified),
            IndexKind::Keyword(topk_rs::proto::v1::control::KeywordIndexType::Exact),
            IndexKind::Semantic,
            IndexKind::None,
        ] {
            assert!(Fold::of(kind).is_none());
        }
    }
}
