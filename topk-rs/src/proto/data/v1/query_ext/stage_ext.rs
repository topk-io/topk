use crate::proto::data::v1::{stage, AggregateExpr, LogicalExpr, Stage};

pub trait IntoSortExprs {
    fn into_sort_exprs(self) -> Vec<stage::sort_stage::SortExpr>;
}

// Sort by field, DESC

impl<'a> IntoSortExprs for &'a str {
    fn into_sort_exprs(self) -> Vec<stage::sort_stage::SortExpr> {
        (LogicalExpr::field(self), stage::sort_stage::SortOrder::Desc).into_sort_exprs()
    }
}

impl IntoSortExprs for String {
    fn into_sort_exprs(self) -> Vec<stage::sort_stage::SortExpr> {
        (LogicalExpr::field(self), stage::sort_stage::SortOrder::Desc).into_sort_exprs()
    }
}

// Sort by expr, DESC

impl IntoSortExprs for LogicalExpr {
    fn into_sort_exprs(self) -> Vec<stage::sort_stage::SortExpr> {
        (self, stage::sort_stage::SortOrder::Desc).into_sort_exprs()
    }
}

// Explicit sort expr and order

impl IntoSortExprs for (LogicalExpr, stage::sort_stage::SortOrder) {
    fn into_sort_exprs(self) -> Vec<stage::sort_stage::SortExpr> {
        let (expr, order) = self;
        vec![stage::sort_stage::SortExpr {
            expr: Some(expr),
            order: order.into(),
        }]
    }
}

impl<const N: usize> IntoSortExprs for [(LogicalExpr, stage::sort_stage::SortOrder); N] {
    fn into_sort_exprs(self) -> Vec<stage::sort_stage::SortExpr> {
        self.into_iter()
            .map(|(expr, order)| stage::sort_stage::SortExpr {
                expr: Some(expr),
                order: order.into(),
            })
            .collect()
    }
}

impl IntoSortExprs for Vec<(LogicalExpr, stage::sort_stage::SortOrder)> {
    fn into_sort_exprs(self) -> Vec<stage::sort_stage::SortExpr> {
        self.into_iter()
            .map(|(expr, order)| stage::sort_stage::SortExpr {
                expr: Some(expr),
                order: order.into(),
            })
            .collect()
    }
}

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

    pub fn offset(offset: u64) -> Self {
        Stage {
            stage: Some(stage::Stage::Offset(stage::OffsetStage { offset })),
        }
    }

    pub fn sort(exprs: impl IntoSortExprs) -> Self {
        Stage {
            stage: Some(stage::Stage::Sort(stage::SortStage {
                exprs: exprs.into_sort_exprs(),
                // Set deprecated fields to default value
                #[allow(deprecated)]
                expr: None,
                #[allow(deprecated)]
                asc: false,
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
