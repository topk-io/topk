use sqlparser::ast::{
    Expr as SqlExpr, Function as SqlFunction, FunctionArg, FunctionArgExpr, FunctionArguments,
};

use crate::Error;

pub trait SqlFunctionExt {
    /// Get the function name.
    fn name(&self) -> String;

    /// Get the arguments of the function.
    fn args(&self) -> Result<Vec<SqlExpr>, Error>;
}

impl SqlFunctionExt for SqlFunction {
    fn name(&self) -> String {
        self.name.to_string()
    }

    fn args(&self) -> Result<Vec<SqlExpr>, Error> {
        match &self.args {
            FunctionArguments::None => Ok(Vec::new()),
            FunctionArguments::Subquery(_) => {
                Err(Error::Unsupported("function call shape".to_string()))
            }
            FunctionArguments::List(list) => list
                .args
                .iter()
                .map(|arg| match arg {
                    FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) => Ok(e.clone()),
                    _ => Err(Error::Unsupported(
                        "named or qualified function argument".to_string(),
                    )),
                })
                .collect(),
        }
    }
}
