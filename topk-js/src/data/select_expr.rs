use napi::bindgen_prelude::*;

use super::{function_expr::FunctionExpression, logical_expr::LogicalExpression};

#[derive(Debug, Clone)]
pub enum SelectExpression {
    Logical { expr: LogicalExpression },
    Function { expr: FunctionExpression },
}

impl Into<topk_rs::expr::select::SelectExpr> for SelectExpression {
    fn into(self) -> topk_rs::expr::select::SelectExpr {
        match self {
            SelectExpression::Logical { expr } => {
                topk_rs::expr::select::SelectExpr::Logical(expr.into())
            }
            SelectExpression::Function { expr } => {
                topk_rs::expr::select::SelectExpr::Function(expr.into())
            }
        }
    }
}

impl FromNapiValue for SelectExpression {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> Result<Self, napi::Status> {
        let env_env = Env::from_raw(env);

        // Create a new Unknown for each instance_of check
        let is_logical_expression = {
            let env_value = Unknown::from_napi_value(env, value)?;
            LogicalExpression::instance_of(env_env, env_value)?
        };

        if is_logical_expression {
            return Ok(SelectExpression::Logical {
                expr: LogicalExpression::from_napi_value(env, value)?,
            });
        }

        let is_function_expression = {
            let object = Object::from_napi_value(env, value)?;

            let type_prop: String = object.get_named_property("type")?;

            match type_prop {
                val if val == "KeywordScore".to_owned() => true,
                val if val == "VectorScore".to_owned() => true,
                val if val == "SemanticSimilarity".to_owned() => true,
                _ => false,
            }
        };

        if is_function_expression {
            return Ok(SelectExpression::Function {
                expr: FunctionExpression::from_napi_value(env, value)?,
            });
        }

        unreachable!("Value must be either a LogicalExpression or FunctionExpression")
    }
}

impl ToNapiValue for SelectExpression {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        match val {
            SelectExpression::Logical { expr } => ToNapiValue::to_napi_value(env, expr),
            SelectExpression::Function { expr } => ToNapiValue::to_napi_value(env, expr),
        }
    }
}
