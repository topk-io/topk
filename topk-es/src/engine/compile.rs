use topk_rs::proto::v1::control::{
    field_index, field_type, field_type_matrix::MatrixValueType, FieldSpec,
    MultiVectorDistanceMetric, VectorDistanceMetric,
};
use topk_rs::proto::v1::data::{FunctionExpr, LogicalExpr, Query as TopkQuery, TextExpr, Value};
use topk_rs::query::{count as count_query, field, filter, fns, not, should};

use super::rank::Ranking;
use super::score::{AnnQuery, AnnTerm, CompiledQuery, Score};
use super::value::ValueExt;
use super::{agg, RANK_ANN, RANK_BM25, RANK_SCORE};
use crate::api::{
    GateQuery, KnnRequest, MatchAllQuery, MatchOperator, MatchValue, Query, QueryVector,
    SearchRequest, TermValue,
};
use crate::{engine::Schema, Error};

pub fn search(
    schema: &Schema,
    mut req: SearchRequest,
) -> Result<(SearchRequest, Vec<(TopkQuery, Option<f32>)>, Vec<TopkQuery>), Error> {
    let mut compiled = Vec::new();
    if let Some(query) = req.query.take() {
        compiled.push((compile_clause(schema, query)?, None, None));
    }
    for knn in req.knn.take().unwrap_or_default() {
        let k = knn.k;
        let threshold = knn.similarity.map(|min| min * knn.boost.unwrap_or(1.0));
        compiled.push((compile_knn(schema, knn)?, Some(k), threshold));
    }
    if compiled.is_empty() {
        let match_all = Query::MatchAll(MatchAllQuery::default());
        compiled.push((compile_clause(schema, match_all)?, None, None));
    }

    if req.rank.is_some() && compiled.len() < 2 {
        return Err(Error::InvalidQuery(
            "\"rank\" requires at least two retrievers (query/knn)".into(),
        ));
    }

    let gate = LogicalExpr::any(compiled.iter().map(|(c, _, _)| c.gate.clone()));

    let window = Ranking::of(&req).window_size();
    let queries = compiled
        .into_iter()
        .map(|(c, k, threshold)| {
            let limit = k.unwrap_or(req.from + req.size).max(window).max(1);
            Ok((lower(schema, &req, c, k.is_some(), limit)?, threshold))
        })
        .collect::<Result<Vec<_>, Error>>()?;

    let agg_queries = req
        .aggs
        .iter()
        .map(|(_, clause)| agg::compile(clause, &gate))
        .collect::<Result<Vec<_>, _>>()?;

    Ok((req, queries, agg_queries))
}

pub fn count(schema: &Schema, query: Option<GateQuery>) -> Result<TopkQuery, Error> {
    Ok(match query {
        Some(q) => filter(compile_clause(schema, q.0)?.gate).count(),
        None => count_query(),
    })
}

fn lower(
    schema: &Schema,
    req: &SearchRequest,
    compiled: CompiledQuery,
    knn: bool,
    limit: u64,
) -> Result<TopkQuery, Error> {
    let score = compiled.score;
    let has_bm25 = score.bm25.is_some();
    let mut query = match score.bm25 {
        Some(text) => filter(text).filter(compiled.gate),
        None => filter(compiled.gate),
    };

    if has_bm25 {
        query = query.select([(RANK_BM25, fns::bm25_score(None, None))]);
    }
    let (query, ann_term) = ann_score(query, schema, &score.anns)?;

    let total = [has_bm25.then(|| field(RANK_BM25)), ann_term, score.expr]
        .into_iter()
        .flatten()
        .reduce(|acc, part| acc.add(part))
        .unwrap_or_else(|| LogicalExpr::literal(0.0f32));

    let query = query.select([(RANK_SCORE, total)]);

    let query = match (knn, req.sort.as_ref()) {
        (false, Some(sort)) => query.sort(field(sort.field.as_str()), sort.asc),
        _ => query.sort(field(RANK_SCORE), false),
    }
    .limit(limit);

    let query = match (req.source.enabled(), req.sort.as_ref()) {
        (true, _) => query.fetch(["*"]),
        (false, Some(sort)) => query.fetch([sort.field.as_str()]),
        (false, None) => query,
    };

    Ok(query)
}

fn compile_knn(schema: &Schema, knn: KnnRequest) -> Result<CompiledQuery, Error> {
    let gate = knn
        .filter
        .into_iter()
        .map(|clause| Ok(compile_clause(schema, clause.0)?.gate))
        .collect::<Result<Vec<_>, Error>>()?
        .into_iter()
        .reduce(LogicalExpr::and)
        .unwrap_or_else(|| LogicalExpr::literal(true));

    Ok(CompiledQuery {
        gate,
        score: Score {
            anns: vec![AnnTerm {
                field: knn.field.as_str().to_string(),
                weight: knn.boost.unwrap_or(1.0),
                query: AnnQuery::Vector {
                    vector: knn.query_vector,
                    num_candidates: knn.num_candidates,
                },
            }],
            ..Score::default()
        },
    })
}

fn constant(gate: LogicalExpr, boost: Option<f32>) -> CompiledQuery {
    CompiledQuery {
        gate,
        score: Score::constant(boost),
    }
}

fn bm25(gate: LogicalExpr, text: TextExpr) -> CompiledQuery {
    CompiledQuery {
        gate,
        score: Score {
            bm25: Some(text),
            ..Score::default()
        },
    }
}

fn compile_clause(schema: &Schema, query: Query) -> Result<CompiledQuery, Error> {
    match query {
        Query::MatchAll(q) => Ok(constant(LogicalExpr::literal(true), q.boost)),
        Query::Match(clause) => {
            let (query, operator, boost) = match clause.value {
                MatchValue::Bare(query) => (query, MatchOperator::Or, None),
                MatchValue::Full(full) => (full.query, full.operator, full.boost),
            };

            let text = should(&query, Some(clause.field.as_str()), boost);

            Ok(bm25(
                match operator {
                    MatchOperator::And => field(clause.field).match_all(query),
                    MatchOperator::Or => field(clause.field).match_any(query),
                },
                text,
            ))
        }
        Query::MultiMatch(m) => {
            let all = matches!(m.operator, MatchOperator::And);
            let boost = m.boost.unwrap_or(1.0);

            let mut text: Option<TextExpr> = None;
            let mut gates = Vec::with_capacity(m.fields.len());
            for f in m.fields {
                let stage = should(&m.query, Some(f.name.as_str()), Some(boost * f.boost));
                text = match text {
                    Some(text) => Some(text.or(stage)),
                    None => Some(stage),
                };
                gates.push(match all {
                    true => field(f.name).match_all(m.query.clone()),
                    false => field(f.name).match_any(m.query.clone()),
                });
            }

            match text {
                Some(text) => Ok(bm25(LogicalExpr::any(gates), text)),
                None => Err(Error::InvalidQuery(
                    "multi_match \"fields\" must not be empty".into(),
                )),
            }
        }
        Query::Term(clause) => {
            let boost = match &clause.value {
                TermValue::Full { boost, .. } => *boost,
                TermValue::Bare(_) => None,
            };
            let field_name = clause.field.as_str().to_string();
            let value = clause.value.value();
            let gate = field(field_name.clone()).eq(value.clone());

            let text_indexed = matches!(
                schema
                    .get(&field_name)
                    .and_then(|spec| spec.index.as_ref())
                    .and_then(|index| index.index.as_ref()),
                Some(field_index::Index::KeywordIndex(_))
            );

            match value.as_string() {
                Some(token) if text_indexed => {
                    Ok(bm25(gate, should(token, Some(&field_name), boost)))
                }
                _ => Ok(constant(gate, boost)),
            }
        }
        Query::Terms(q) => Ok(constant(field(q.field).in_(q.values), q.boost)),
        Query::Ids(q) => Ok(constant(
            field("_id").in_(Value::list(
                q.values.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
            )),
            q.boost,
        )),
        Query::Prefix(c) => Ok(constant(
            field(c.field).starts_with(String::from(&c.value)),
            None,
        )),
        Query::Regexp(c) => {
            let flags = c.value.case_insensitive().then(|| "i");
            Ok(constant(
                field(c.field).regexp_match(String::from(&c.value), flags),
                None,
            ))
        }
        Query::Range(clause) => {
            let boost = clause.value.boost;
            let mut exprs = Vec::new();
            if let Some(v) = clause.value.gte {
                exprs.push(field(clause.field.clone()).gte(v.into_inner()));
            }
            if let Some(v) = clause.value.gt {
                exprs.push(field(clause.field.clone()).gt(v.into_inner()));
            }
            if let Some(v) = clause.value.lte {
                exprs.push(field(clause.field.clone()).lte(v.into_inner()));
            }
            if let Some(v) = clause.value.lt {
                exprs.push(field(clause.field.clone()).lt(v.into_inner()));
            }
            if exprs.is_empty() {
                return Err(Error::InvalidQuery("Range query has no bounds".into()));
            }
            Ok(constant(LogicalExpr::all(exprs), boost))
        }
        Query::Exists(q) => Ok(constant(field(q.field).is_not_null(), None)),
        Query::Bool(query) => {
            if query.is_empty() {
                return Ok(constant(LogicalExpr::literal(true), query.boost));
            }

            let boost = query.boost.unwrap_or(1.0);
            let required_empty = query.must.is_empty() && query.filter.is_empty();

            let mut gates = Vec::new();
            let mut scores = Vec::new();

            for clause in query.must {
                let compiled = compile_clause(schema, clause)?;
                gates.push(compiled.gate);
                scores.push(compiled.score);
            }

            gates.extend(
                query
                    .filter
                    .into_iter()
                    .map(|clause| Ok(compile_clause(schema, clause.0)?.gate))
                    .collect::<Result<Vec<_>, Error>>()?,
            );

            let must_not = query
                .must_not
                .into_iter()
                .map(|clause| Ok(compile_clause(schema, clause.0)?.gate))
                .collect::<Result<Vec<_>, Error>>()?;
            if !must_not.is_empty() {
                gates.push(not(LogicalExpr::any(must_not)));
            }

            if !query.should.is_empty() {
                let mut should_gates = Vec::with_capacity(query.should.len());
                for clause in query.should {
                    let mut compiled = compile_clause(schema, clause)?;

                    if let Some(expr) = compiled.score.expr.take() {
                        compiled.score.expr = Some(
                            compiled
                                .gate
                                .clone()
                                .choose(expr, LogicalExpr::literal(0.0f32)),
                        );
                    }

                    scores.push(compiled.score);
                    should_gates.push(compiled.gate);
                }
                if required_empty {
                    gates.push(LogicalExpr::any(should_gates));
                }
            }

            Ok(CompiledQuery {
                gate: LogicalExpr::all(gates),
                score: Score::sum(scores, boost),
            })
        }
        Query::Semantic(s) => Ok(CompiledQuery {
            gate: LogicalExpr::literal(true),
            score: Score {
                anns: vec![AnnTerm {
                    field: s.field.as_str().to_string(),
                    weight: s.boost.unwrap_or(1.0),
                    query: AnnQuery::Semantic(s.query),
                }],
                ..Score::default()
            },
        }),
    }
}

fn ann_score(
    mut query: TopkQuery,
    schema: &Schema,
    anns: &[AnnTerm],
) -> Result<(TopkQuery, Option<LogicalExpr>), Error> {
    // Every ANN term gets its own scorer select.
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
                let spec = match spec.index.as_ref().and_then(|i| i.index.as_ref()) {
                    Some(field_index::Index::SemanticIndex(_)) => {
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

        let higher_is_better = match spec.index.as_ref().and_then(|i| i.index.as_ref()) {
            Some(field_index::Index::VectorIndex(v)) => match v.metric() {
                VectorDistanceMetric::Cosine | VectorDistanceMetric::DotProduct => true,
                VectorDistanceMetric::Euclidean | VectorDistanceMetric::Hamming => false,
                VectorDistanceMetric::Unspecified => return Err(not_knn_searchable(&ann.field)),
            },
            Some(field_index::Index::MultiVectorIndex(mv)) => match mv.metric() {
                MultiVectorDistanceMetric::Maxsim => true,
                _ => return Err(not_knn_searchable(&ann.field)),
            },
            _ => return Err(not_knn_searchable(&ann.field)),
        };

        let rank_ann = format!("{RANK_ANN}_{index}");
        query = query.select([(rank_ann.as_str(), scorer)]);
        let folded = match higher_is_better {
            true => field(rank_ann.as_str()),
            false => LogicalExpr::literal(1.0f32)
                .div(LogicalExpr::literal(1.0f32).add(field(rank_ann.as_str()))),
        };

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
    let index = spec
        .index
        .as_ref()
        .and_then(|i| i.index.as_ref())
        .ok_or_else(|| not_knn_searchable(field_name))?;

    match index {
        field_index::Index::VectorIndex(_) => match query_vector {
            QueryVector::Flat(v) => Ok(fns::vector_distance(
                field_name,
                query_value(field_name, spec, v)?,
            )),
            QueryVector::Matrix(_) => Err(Error::InvalidQuery(format!(
                "\"{field_name}\" is a dense_vector field; query_vector must be a flat array"
            ))),
        },
        field_index::Index::MultiVectorIndex(_) => match query_vector {
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
