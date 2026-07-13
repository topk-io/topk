use crate::proto::data::v1::{
    query_ext::stage_ext::IntoSortExprs,
    stage::{self, sort_stage::SortOrder},
    AggregateExpr, LogicalExpr, Query, Stage,
};

pub mod expr_ext;
pub mod stage_ext;

impl Query {
    pub fn new(stages: Vec<Stage>) -> Self {
        Query { stages }
    }

    pub fn select(
        mut self,
        exprs: impl IntoIterator<
            Item = (
                impl Into<String>,
                impl Into<stage::select_stage::SelectExpr>,
            ),
        >,
    ) -> Self {
        self.stages.push(Stage::select(exprs));
        self
    }

    pub fn filter(mut self, expr: impl Into<stage::filter_stage::FilterExpr>) -> Self {
        self.stages.push(Stage::filter(expr));
        self
    }

    /// Use `.sort(expr, asc).limit(k)` instead.
    #[deprecated(note = "Use `.sort(expr, asc).limit(k)` instead")]
    pub fn topk(mut self, expr: LogicalExpr, k: u64, asc: bool) -> Self {
        self.stages.push(Stage::sort([(
            expr,
            asc.then_some(SortOrder::Asc).unwrap_or(SortOrder::Desc),
        )]));
        self.stages.push(Stage::limit(k));
        self
    }

    pub fn count(mut self) -> Self {
        self.stages.push(Stage::count());
        self
    }

    pub fn limit(mut self, k: u64) -> Self {
        self.stages.push(Stage::limit(k));
        self
    }

    pub fn offset(mut self, offset: u64) -> Self {
        self.stages.push(Stage::offset(offset));
        self
    }

    pub fn sort(mut self, exprs: impl IntoSortExprs) -> Self {
        self.stages.push(Stage::sort(exprs));
        self
    }

    pub fn fetch(mut self, fields: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.stages.push(Stage::fetch(fields));
        self
    }

    pub fn group_by(
        mut self,
        keys: impl IntoIterator<Item = (impl Into<String>, impl Into<LogicalExpr>)>,
        aggs: impl IntoIterator<Item = (impl Into<String>, impl Into<AggregateExpr>)>,
    ) -> Self {
        self.stages.push(Stage::group_by(keys, aggs));
        self
    }
}
