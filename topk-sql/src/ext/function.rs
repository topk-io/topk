use sqlparser::ast::{
    Expr as SqlExpr, Function as SqlFunction, FunctionArg, FunctionArgExpr, FunctionArguments,
};

use crate::Error;

pub trait SqlFunctionExt {
    /// Get the function name.
    fn name(&self) -> String;

    /// Get the arguments of the function.
    fn args(&self) -> Result<Vec<SqlExpr>, Error>;

    fn is_count(&self) -> bool;

    fn matches_args<F>(&self, check: F) -> bool
    where
        F: Fn(&[FunctionArg]) -> bool;
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

    fn is_count(&self) -> bool {
        self.name.0.len() == 1 && self.name.0[0].value.eq_ignore_ascii_case("count")
    }

    fn matches_args<F>(&self, check: F) -> bool
    where
        F: Fn(&[FunctionArg]) -> bool,
    {
        match &self.args {
            FunctionArguments::List(list) if list.duplicate_treatment.is_none() => {
                check(list.args.as_slice())
            }
            _ => false,
        }
    }
}
