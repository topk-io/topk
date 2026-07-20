use topk_rs::proto::v1::control::KeywordIndexType;
use topk_rs::proto::v1::data::{LogicalExpr, Query as TopkQuery, TextExpr, Value};
use topk_rs::query::{count as count_query, field, filter, fns, not, should, SortOrder};

use super::field::{ensure_aggregatable, IndexKind};
use super::rank::Ranking;
use super::score::{ann_score, AnnQuery, AnnTerm, CompiledQuery, Score};
use super::{agg, RANK_BM25, RANK_SCORE};
use crate::api::{
    AggClause, AggType, FieldName, GateQuery, KnnRequest, MatchAllQuery, MatchOperator, MatchValue,
    Query, SearchRequest, SortField, SortTarget, TermValue,
};
use crate::value::ValueExt;

use crate::{engine::Schema, Error};

fn validate_agg_fields(schema: &Schema, clause: &AggClause) -> Result<(), Error> {
    match &clause.ty {
        AggType::Terms(terms) => ensure_aggregatable(schema, terms.field.as_str())?,
        AggType::Sum(m) | AggType::Avg(m) | AggType::Min(m) | AggType::Max(m) => {
            ensure_aggregatable(schema, m.field.as_str())?
        }
        AggType::ValueCount(_) => {}
    }
    for sub in clause.aggs.iter().flatten() {
        validate_agg_fields(schema, sub.1)?;
    }
    Ok(())
}

pub fn search(
    schema: &Schema,
    mut req: SearchRequest,
) -> Result<(SearchRequest, Vec<TopkQuery>, Vec<TopkQuery>), Error> {
    let mut compiled = Vec::new();
    if let Some(query) = req.query.take() {
        compiled.push((compile_clause(schema, query)?, None));
    }
    for knn in req.knn.take().unwrap_or_default() {
        let k = knn.k;
        compiled.push((compile_knn(schema, knn)?, Some(k)));
    }
    if compiled.is_empty() {
        let match_all = Query::MatchAll(MatchAllQuery::default());
        compiled.push((compile_clause(schema, match_all)?, None));
    }

    if req.rank.is_some() && compiled.len() < 2 {
        return Err(Error::InvalidQuery(
            "\"rank\" requires at least two retrievers (query/knn)".into(),
        ));
    }

    if req.rank.is_some() && req.size == 0 {
        return Err(Error::InvalidQuery(
            "[rank] requires [size] greater than [0]".into(),
        ));
    }

    if let Some(sort) = req.sort.as_ref() {
        for name in sort.iter().filter_map(SortField::field_name) {
            ensure_aggregatable(schema, name.as_str())?;
        }
    }
    for clause in req.aggs.values() {
        validate_agg_fields(schema, clause)?;
    }

    let gate = LogicalExpr::any(compiled.iter().map(|(c, _)| c.gate.clone()));

    let window = Ranking::of(&req).window_size();
    let queries = compiled
        .into_iter()
        .map(|(c, k)| {
            // A knn retriever contributes exactly its `k` nearest neighbours. Only
            // non-knn retrievers fetch up to the rank window: an ANN query returns
            // its top `limit` unconditionally, so expanding a knn's limit to the
            // window pulls the whole index in as candidates, giving unrelated docs
            // phantom RRF scores.
            let limit = match k {
                Some(k) => k,
                None => (req.from + req.size).max(window),
            }
            .max(1);
            lower(schema, &req, c, k.is_some(), limit)
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
        (false, Some(sort)) => {
            let mut exprs = sort
                .iter()
                .map(|f| match &f.target {
                    // `_score` is the selected rank field, not a document field.
                    SortTarget::Score => (field(RANK_SCORE), f.order()),
                    SortTarget::Field(name) => (field(name.as_str()), f.order()),
                })
                .collect::<Vec<_>>();

            // The engine drops docs whose every sort key is null (and the
            // single-key collector drops any null-keyed doc); ES retains
            // them, sorted last. Pad with a constant key — never null, so
            // the null-retaining multi collector runs and `non_null > 0`
            // holds for every doc. No room at the 8-expr engine cap, where
            // all-null docs are still dropped.
            if exprs.len() < crate::api::MAX_SORT_FIELDS {
                exprs.push((LogicalExpr::literal(0u32), SortOrder::Asc));
            }

            query.sort(exprs)
        }
        _ => query.sort(RANK_SCORE),
    }
    .limit(limit);

    let query = match (req.source.enabled(), req.sort.as_ref()) {
        (true, _) => query.fetch(["*"]),
        (false, Some(sort)) => query.fetch(
            sort.iter()
                .filter_map(SortField::field_name)
                .map(FieldName::as_str),
        ),
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
                cutoff: knn.similarity,
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
            if !value.is_scalar() {
                return Err(Error::InvalidQuery(format!(
                    "[term] query does not support a non-scalar value for field [{field_name}]"
                )));
            }
            let token = value.as_string().map(str::to_string);

            match (
                token,
                schema
                    .get(&field_name)
                    .map(IndexKind::from)
                    .unwrap_or(IndexKind::None),
            ) {
                // Exact keyword: exact-match gate plus IDF score from a verbatim
                // text probe (the router does not analyze exact fields).
                (Some(token), IndexKind::Keyword(KeywordIndexType::Exact)) => Ok(bm25(
                    field(field_name.clone()).eq(value),
                    should(&token, Some(&field_name), boost),
                )),
                // Analyzed text: ES `term` matches an indexed token, so gate on a
                // text match (router analyzes the token) rather than exact scalar
                // equality, which would never match a tokenized value.
                (Some(token), IndexKind::Keyword(KeywordIndexType::Text)) => Ok(bm25(
                    field(field_name.clone()).match_any(token.clone()),
                    should(&token, Some(&field_name), boost),
                )),
                // Non-string values or non-text fields: exact equality, constant
                // query-context score.
                _ => Ok(constant(field(field_name).eq(value), boost)),
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
            // A bound-less range is ES's field-exists check.
            if exprs.is_empty() {
                return Ok(constant(field(clause.field).is_not_null(), boost));
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
        // An empty query has nothing to embed; ES fails it at the inference call.
        Query::Semantic(s) if s.query.trim().is_empty() => Err(Error::InvalidQuery(
            "[semantic] query must not be empty".into(),
        )),
        Query::Semantic(s) => Ok(CompiledQuery {
            gate: LogicalExpr::literal(true),
            score: Score {
                anns: vec![AnnTerm {
                    field: s.field.as_str().to_string(),
                    weight: s.boost.unwrap_or(1.0),
                    cutoff: None,
                    query: AnnQuery::Semantic(s.query),
                }],
                ..Score::default()
            },
        }),
    }
}
