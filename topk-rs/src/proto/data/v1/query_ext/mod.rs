use crate::proto::data::v1::{stage, LogicalExpr, Query, Stage};

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

    pub fn topk(mut self, expr: LogicalExpr, k: u64, asc: bool) -> Self {
        self.stages.push(Stage::sort(expr, asc));
        self.stages.push(Stage::limit(k));
        self
    }

    pub fn count(mut self) -> Self {
        self.stages.push(Stage::count());
        self
    }

    pub fn rerank(
        mut self,
        model: Option<String>,
        query: Option<String>,
        fields: Vec<String>,
        topk_multiple: Option<u32>,
    ) -> Self {
        self.stages
            .push(Stage::rerank(model, query, fields, topk_multiple));
        self
    }

    pub fn limit(mut self, k: u64) -> Self {
        self.stages.push(Stage::limit(k));
        self
    }

    pub fn sort(mut self, expr: LogicalExpr, asc: bool) -> Self {
        self.stages.push(Stage::sort(expr, asc));
        self
    }
}
