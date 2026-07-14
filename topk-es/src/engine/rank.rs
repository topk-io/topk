use std::cmp::{Ordering, Reverse};
use std::collections::HashMap;

use topk_rs::json::Value as JsonValue;
use topk_rs::proto::v1::data::{Document, Value};

use super::doc::decode;
use super::value::OrdValue;
use super::RANK_SCORE;
use crate::api::{DocId, Hit, SearchRequest, SortClause};
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
    fields: HashMap<String, Value>,
}

impl Candidate {
    fn sort_key(&self, sort: &SortClause) -> SortKey {
        SortKey(
            sort.iter()
                .map(|f| {
                    let value = self
                        .fields
                        .get(f.field.as_str())
                        .filter(|value| value.as_null().is_none())
                        .cloned();
                    match (value, f.asc) {
                        (None, _) => SortKeyPart::Missing,
                        (Some(value), true) => SortKeyPart::Asc(OrdValue(value)),
                        (Some(value), false) => SortKeyPart::Desc(Reverse(OrdValue(value))),
                    }
                })
                .collect(),
        )
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct SortKey(Vec<SortKeyPart>);

impl SortKey {
    fn into_json(self) -> Vec<JsonValue> {
        self.0
            .into_iter()
            .map(|part| {
                JsonValue::from(match part {
                    SortKeyPart::Asc(value) => value.0,
                    SortKeyPart::Desc(Reverse(value)) => value.0,
                    SortKeyPart::Missing => Value::null(),
                })
            })
            .collect()
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum SortKeyPart {
    Asc(OrdValue),
    Desc(Reverse<OrdValue>),
    Missing,
}

fn combine(
    req: &SearchRequest,
    results: Vec<(Option<f32>, Vec<Document>)>,
) -> Result<Vec<(f32, Candidate)>, Error> {
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
                fields: doc.fields,
            });
        }
        groups.push(members);
    }

    let totals = Ranking::of(req).consume(groups);

    Ok(totals
        .into_iter()
        .filter_map(|(id, score)| by_id.remove(&id).map(|candidate| (score, candidate)))
        .collect())
}

fn to_hits(req: &SearchRequest, candidates: Vec<(f32, Candidate)>) -> Vec<Hit> {
    let mut candidates: Vec<(Option<SortKey>, f32, Candidate)> = candidates
        .into_iter()
        .map(|(score, candidate)| {
            let key = req.sort.as_ref().map(|sort| candidate.sort_key(sort));
            (key, score, candidate)
        })
        .collect();

    match &req.sort {
        Some(_) => candidates.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.2.id.cmp(&b.2.id))),
        None => {
            candidates.sort_by(|a, b| score_desc((a.1, a.2.id.as_str()), (b.1, b.2.id.as_str())))
        }
    }

    let page = (req.from as usize).min(candidates.len());
    candidates.drain(..page);
    candidates.truncate(req.size as usize);

    let scores = req.sort.is_none() || req.track_scores;
    candidates
        .into_iter()
        .map(|(key, score, candidate)| Hit {
            score: scores.then_some(score),
            sort: key.map(SortKey::into_json),
            source: req
                .source
                .enabled()
                .then(|| decode(&req.source, candidate.fields)),
            id: candidate.id,
        })
        .collect()
}

fn score_desc(a: (f32, &str), b: (f32, &str)) -> Ordering {
    b.0.partial_cmp(&a.0)
        .unwrap_or(Ordering::Equal)
        .then_with(|| a.1.cmp(b.1))
}
