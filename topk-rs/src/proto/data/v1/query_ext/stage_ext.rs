use crate::proto::data::v1::{stage, AggregateExpr, LogicalExpr, Stage};

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

    /// Use `Stage::sort` + `Stage::limit` instead.
    // #[deprecated(note = "Use `Stage::sort` + `Stage::limit` instead")]
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

    pub fn limit(k: u64) -> Self {
        Stage {
            stage: Some(stage::Stage::Limit(stage::LimitStage { k })),
        }
    }

    pub fn sort(expr: LogicalExpr, asc: bool) -> Self {
        Stage {
            stage: Some(stage::Stage::Sort(stage::SortStage {
                expr: Some(expr),
                asc,
            })),
        }
    }

    pub fn fetch(fields: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Stage {
            stage: Some(stage::Stage::Fetch(stage::FetchStage {
                fields: fields.into_iter().map(|s| s.into()).collect(),
            })),
        }
    }

    /// Group documents by one or more key expressions and compute aggregations for each group.
    pub fn group_by(
        keys: impl IntoIterator<Item = (impl Into<String>, impl Into<LogicalExpr>)>,
        aggs: impl IntoIterator<Item = (impl Into<String>, impl Into<AggregateExpr>)>,
    ) -> Self {
        Stage {
            stage: Some(stage::Stage::GroupBy(stage::GroupByStage {
                keys: keys
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
                aggs: aggs
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
            })),
        }
    }
}
