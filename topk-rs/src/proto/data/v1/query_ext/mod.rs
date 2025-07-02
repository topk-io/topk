use crate::proto::data::v1::{stage, LogicalExpr, Query, Stage};

pub mod expr_ext;
pub mod stage_ext;

impl Query {
    pub fn new(stages: Vec<Stage>) -> Self {
        Query { stages }
    }

    pub fn select(
        &self,
        exprs: impl IntoIterator<
            Item = (
                impl Into<String>,
                impl Into<stage::select_stage::SelectExpr>,
            ),
        >,
    ) -> Self {
        let mut stages = self.stages.clone();
        stages.push(Stage::select(exprs));
        Query { stages }
    }

    pub fn filter(&self, expr: impl Into<stage::filter_stage::FilterExpr>) -> Self {
        let mut stages = self.stages.clone();
        stages.push(Stage::filter(expr));
        Query { stages }
    }

    pub fn topk(&self, expr: LogicalExpr, k: u64, asc: bool) -> Self {
        let mut stages = self.stages.clone();
        stages.push(Stage::topk(expr, k, asc));
        Query { stages }
    }

    pub fn count(&self) -> Self {
        let mut stages = self.stages.clone();
        stages.push(Stage::count());
        Query { stages }
    }

    pub fn rerank(
        &self,
        model: Option<String>,
        query: Option<String>,
        fields: Vec<String>,
        topk_multiple: Option<u32>,
    ) -> Self {
        let mut stages = self.stages.clone();
        stages.push(Stage::rerank(model, query, fields, topk_multiple));
        Query { stages }
    }
}
