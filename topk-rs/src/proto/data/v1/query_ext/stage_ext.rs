use crate::proto::data::v1::{stage, LogicalExpr, Stage};

impl Stage {
    pub fn select(
        exprs: impl IntoIterator<
            Item = (
                impl Into<String>,
                impl Into<stage::select_stage::SelectExpr>,
            ),
        >,
    ) -> Self {
        Stage {
            stage: Some(stage::Stage::Select(stage::SelectStage {
                exprs: exprs
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
            })),
        }
    }

    pub fn filter(expr: impl Into<stage::filter_stage::FilterExpr>) -> Self {
        Stage {
            stage: Some(stage::Stage::Filter(stage::FilterStage {
                expr: Some(expr.into()),
            })),
        }
    }

    pub fn topk(expr: LogicalExpr, k: u64, asc: bool) -> Self {
        Stage {
            stage: Some(stage::Stage::TopK(stage::TopKStage {
                expr: Some(expr),
                k,
                asc,
            })),
        }
    }

    pub fn count() -> Self {
        Stage {
            stage: Some(stage::Stage::Count(stage::CountStage {})),
        }
    }

    pub fn rerank(
        model: Option<String>,
        query: Option<String>,
        fields: Vec<String>,
        topk_multiple: Option<u32>,
    ) -> Self {
        Stage {
            stage: Some(stage::Stage::Rerank(stage::RerankStage {
                model,
                query,
                fields,
                topk_multiple,
            })),
        }
    }
}
