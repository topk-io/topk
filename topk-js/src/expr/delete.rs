use napi::bindgen_prelude::*;

use crate::expr::logical::LogicalExpression;

#[derive(Debug, Clone)]
pub enum DeleteExpression {
    Ids(Vec<String>),
    Filter(LogicalExpression),
}

impl FromNapiValue for DeleteExpression {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        // Try parsing as a list of document IDs
        if let Ok(ids) = Vec::<String>::from_napi_value(env, value) {
            return Ok(DeleteExpression::Ids(ids));
        }

        // Try parsing as a delete filter expression
        if let Ok(expr) = crate::try_cast_ref!(env, value, LogicalExpression) {
            return Ok(DeleteExpression::Filter(expr.clone()));
        }

        Err(napi::Error::from_reason(
            "Unsupported delete expression value",
        ))
    }
}

impl From<DeleteExpression> for topk_rs::proto::v1::data::DeleteDocumentsRequest {
    fn from(expr: DeleteExpression) -> Self {
        match expr {
            DeleteExpression::Filter(expr) => {
                topk_rs::proto::v1::data::DeleteDocumentsRequest::filter(expr)
            }
            DeleteExpression::Ids(ids) => {
                topk_rs::proto::v1::data::DeleteDocumentsRequest::ids(ids)
            }
        }
    }
}
