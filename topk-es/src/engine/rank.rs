use std::cmp::Ordering;
use std::collections::HashMap;

use topk_rs::proto::v1::data::{Document, Value};

use super::doc::decode;
use super::value::ValueExt;
use super::RANK_SCORE;
use crate::api::{DocId, Hit, SearchRequest};
use crate::Error;

pub fn fuse(
    req: &SearchRequest,
    results: Vec<(Option<f32>, Vec<Document>)>,
) -> Result<Vec<Hit>, Error> {
    let candidates = combine(req, results)?;
    Ok(to_hits(req, candidates))
}

pub enum Ranking {
    Sum,
    Rrf {
        rank_constant: f32,
        rank_window_size: u64,
    },
}

impl Ranking {
    // The request's ranking strategy, defaulting to a plain weighted sum
    // when no rank clause is given.
    pub fn of(req: &SearchRequest) -> Ranking {
        match &req.rank {
            Some(clause) => Ranking::Rrf {
                rank_constant: clause.rrf.rank_constant.unwrap_or(60.0),
                rank_window_size: clause
                    .rrf
                    .rank_window_size
                    .unwrap_or((req.from + req.size).max(10)),
            },
            None => Ranking::Sum,
        }
    }

    pub fn window_size(&self) -> u64 {
        match self {
            Ranking::Sum => 0,
            Ranking::Rrf {
                rank_window_size, ..
            } => *rank_window_size,
        }
    }

    fn consume(&self, groups: Vec<Vec<(DocId, f32)>>) -> HashMap<DocId, f32> {
        let mut totals: HashMap<DocId, f32> = HashMap::new();
        match self {
            Ranking::Sum => {
                for (id, value) in groups.into_iter().flatten() {
                    *totals.entry(id).or_insert(0.0) += value;
                }
            }
            Ranking::Rrf {
                rank_constant,
                rank_window_size,
            } => {
                for mut ranked in groups {
                    ranked.sort_by(|a, b| score_desc((a.1, a.0.as_str()), (b.1, b.0.as_str())));
                    for (position, (id, _)) in ranked
                        .into_iter()
                        .take(*rank_window_size as usize)
                        .enumerate()
                    {
                        *totals.entry(id).or_insert(0.0) +=
                            1.0 / (rank_constant + (position + 1) as f32);
                    }
                }
            }
        }
        totals
    }
}

struct Candidate {
    id: DocId,
    score: f32,
    fields: HashMap<String, Value>,
}

fn combine(
    req: &SearchRequest,
    results: Vec<(Option<f32>, Vec<Document>)>,
) -> Result<Vec<Candidate>, Error> {
    let mut by_id: HashMap<DocId, Candidate> = HashMap::new();

    let mut groups: Vec<Vec<(DocId, f32)>> = Vec::with_capacity(results.len());
    for (threshold, docs) in results {
        let mut members = Vec::with_capacity(docs.len());
        for mut doc in docs {
            let id = DocId::try_from(
                doc.id()
                    .map_err(|e| Error::Internal(e.to_string()))?
                    .to_string(),
            )?;
            let score = doc
                .fields
                .remove(RANK_SCORE)
                .and_then(|v| v.as_f32())
                .unwrap_or(0.0);
            if threshold.map(|min| score < min).unwrap_or(false) {
                continue;
            }
            members.push((id.clone(), score));
            by_id.entry(id.clone()).or_insert(Candidate {
                id,
                score,
                fields: doc.fields,
            });
        }
        groups.push(members);
    }

    let totals = Ranking::of(req).consume(groups);

    Ok(totals
        .into_iter()
        .filter_map(|(id, score)| {
            by_id.remove(&id).map(|mut candidate| {
                candidate.score = score;
                candidate
            })
        })
        .collect())
}

fn to_hits(req: &SearchRequest, mut candidates: Vec<Candidate>) -> Vec<Hit> {
    match &req.sort {
        Some(sort) => candidates.sort_by(|a, b| {
            let ordering = match (
                sort_key(a, sort.field.as_str()),
                sort_key(b, sort.field.as_str()),
            ) {
                (Some(x), Some(y)) => match sort.asc {
                    true => compare_value(x, y),
                    false => compare_value(x, y).reverse(),
                },
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            };
            ordering.then_with(|| a.id.cmp(&b.id))
        }),
        None => candidates
            .sort_by(|a, b| score_desc((a.score, a.id.as_str()), (b.score, b.id.as_str()))),
    }

    let page = (req.from as usize).min(candidates.len());
    candidates.drain(..page);
    candidates.truncate(req.size as usize);

    let scores = req.sort.is_none() || req.track_scores;
    candidates
        .into_iter()
        .map(|candidate| Hit {
            score: scores.then_some(candidate.score),
            source: req
                .source
                .enabled()
                .then(|| decode(&req.source, candidate.fields)),
            id: candidate.id,
        })
        .collect()
}

fn sort_key<'a>(candidate: &'a Candidate, field: &str) -> Option<&'a Value> {
    candidate
        .fields
        .get(field)
        .filter(|value| value.as_null().is_none())
}

fn score_desc(a: (f32, &str), b: (f32, &str)) -> Ordering {
    b.0.partial_cmp(&a.0)
        .unwrap_or(Ordering::Equal)
        .then_with(|| a.1.cmp(b.1))
}

fn compare_value(a: &Value, b: &Value) -> Ordering {
    if let (Some(x), Some(y)) = (a.number(), b.number()) {
        return x.partial_cmp(&y).unwrap_or(Ordering::Equal);
    }
    if let (Some(x), Some(y)) = (a.as_string(), b.as_string()) {
        return x.cmp(y);
    }
    if let (Some(x), Some(y)) = (a.as_bool(), b.as_bool()) {
        return x.cmp(&y);
    }
    Ordering::Equal
}
