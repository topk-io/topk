use topk_rs::proto::v1::data::{LogicalExpr, TextExpr};

use crate::api::QueryVector;

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
