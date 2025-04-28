use super::Stage;
use crate::expr::{filter::FilterExpr, logical::LogicalExpr, select::SelectExpr};

#[derive(Clone)]
pub struct Query {
    stages: Vec<Stage>,
}

impl Query {
    pub fn new(stages: Vec<Stage>) -> Self {
        Self { stages }
    }

    pub fn select<S, E>(&self, exprs: impl IntoIterator<Item = (S, E)>) -> Self
    where
        S: Into<String>,
        E: Into<SelectExpr>,
    {
        let mut query = self.clone();
        query.stages.push(Stage::Select {
            exprs: exprs
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        });
        query
    }

    pub fn filter(&self, expr: impl Into<FilterExpr>) -> Self {
        let mut query = self.clone();
        query.stages.push(Stage::Filter {
            expr: expr.into().into(),
        });
        query
    }

    pub fn topk(&self, expr: impl Into<LogicalExpr>, limit: u64, asc: bool) -> Self {
        let mut query = self.clone();
        query.stages.push(Stage::TopK {
            expr: expr.into().into(),
            k: limit,
            asc,
        });
        query
    }

    pub fn count(&self) -> Self {
        let mut query = self.clone();
        query.stages.push(Stage::Count {});
        query
    }

    pub fn rerank(
        &self,
        model: Option<String>,
        query: Option<String>,
        fields: Vec<String>,
        topk_multiple: Option<u32>,
    ) -> Self {
        let mut new_query = self.clone();
        new_query.stages.push(Stage::Rerank {
            model,
            query,
            fields,
            topk_multiple,
        });
        new_query
    }
}

impl From<Query> for topk_protos::v1::data::Query {
    fn from(query: Query) -> Self {
        Self {
            stages: query.stages.into_iter().map(|s| s.into()).collect(),
        }
    }
}
