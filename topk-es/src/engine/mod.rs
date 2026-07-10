use std::collections::HashMap;

use topk_rs::proto::v1::control::FieldSpec;

pub type Schema = HashMap<String, FieldSpec>;

pub mod agg;
pub mod compile;
pub mod doc;
pub mod rank;
pub mod score;
pub mod value;

const RANK_PREFIX: &str = "topk_es_rank_";
const RANK_SCORE: &str = "topk_es_rank_score";
const RANK_BM25: &str = "topk_es_rank_bm25";
const RANK_ANN: &str = "topk_es_rank_ann";
