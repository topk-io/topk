use topk_rs::proto::v1::data::{FunctionExpr, LogicalExpr, TextExpr, Value};

mod filter;
mod function;
mod logical;
mod regexp;
mod select;
mod typed;
mod value;

#[derive(Clone, Debug)]
pub enum Expr {
    Literal(Value),
    Logical(LogicalExpr),
    Text(TextExpr),
    Function(FunctionExpr),
}
