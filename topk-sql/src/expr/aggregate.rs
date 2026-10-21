use sqlparser::ast::{
    DuplicateTreatment, Expr as SqlExpr, Function as SqlFunction, FunctionArg, FunctionArgExpr,
    FunctionArguments,
};

use topk_rs::proto::v1::data::{AggregateExpr, Value};

use crate::{Error, FromSql, SqlExprExt, SqlFunctionExt};

impl FromSql<SqlFunction> for AggregateExpr {
    fn from_sql(func: SqlFunction) -> Result<AggregateExpr, Error> {
        let name = func.name();
        let key = name.to_ascii_lowercase();

        validate_aggregate_call(&func, &name)?;

        Ok(match key.as_str() {
            "count" => {
                if func.matches_args(|args| {
                    matches!(args, [FunctionArg::Unnamed(FunctionArgExpr::Wildcard)])
                }) {
                    AggregateExpr::count(None)
                } else {
                    AggregateExpr::count(Some(single_field_arg(&func, &name)?))
                }
            }
            "sum" => AggregateExpr::sum(single_field_arg(&func, &name)?),
            "min" => AggregateExpr::min(single_field_arg(&func, &name)?),
            "max" => AggregateExpr::max(single_field_arg(&func, &name)?),
            "avg" => AggregateExpr::avg(single_field_arg(&func, &name)?),
            "approx_percentile" => {
                let (field, q) = quantile_args(&func, &name)?;
                AggregateExpr::quantile(field, q)
            }
            "approx_count_distinct" => {
                AggregateExpr::count_distinct(single_field_arg(&func, &name)?)
            }
            _ => {
                return Err(Error::Unsupported(format!(
                    "`{name}` is not a supported aggregate function \
                     (expected COUNT/SUM/MIN/MAX/AVG/APPROX_PERCENTILE/APPROX_COUNT_DISTINCT)"
                )));
            }
        })
    }
}

fn validate_aggregate_call(func: &SqlFunction, name: &str) -> Result<(), Error> {
    if matches!(
        &func.args,
        FunctionArguments::List(list)
            if matches!(list.duplicate_treatment, Some(DuplicateTreatment::Distinct))
    ) {
        return Err(Error::Unsupported(format!(
            "{name}: DISTINCT aggregates are not supported"
        )));
    }

    if matches!(
        &func.args,
        FunctionArguments::List(list) if !list.clauses.is_empty()
    ) {
        return Err(Error::Unsupported(format!(
            "{name}: aggregate argument clauses are not supported"
        )));
    }

    if func.filter.is_some() {
        return Err(Error::Unsupported(format!(
            "{name}: aggregate FILTER clause is not supported"
        )));
    }

    if func.over.is_some() {
        return Err(Error::Unsupported(format!(
            "{name}: aggregate OVER clause is not supported"
        )));
    }

    if !func.within_group.is_empty() {
        return Err(Error::Unsupported(format!(
            "{name}: WITHIN GROUP is not supported"
        )));
    }

    if func.null_treatment.is_some() {
        return Err(Error::Unsupported(format!(
            "{name}: NULL treatment is not supported"
        )));
    }

    if !matches!(func.parameters, FunctionArguments::None) {
        return Err(Error::Unsupported(format!(
            "{name}: aggregate parameters are not supported"
        )));
    }

    Ok(())
}

fn single_field_arg(func: &SqlFunction, name: &str) -> Result<String, Error> {
    let args = func.args()?;
    let [arg]: [SqlExpr; 1] = args.try_into().map_err(|v: Vec<SqlExpr>| {
        Error::Invalid(format!("{name}: expected 1 argument, got {}", v.len()))
    })?;
    arg.as_ident()
        .ok_or_else(|| Error::Invalid(format!("{name}: argument must be a column name")))
}

fn quantile_args(func: &SqlFunction, name: &str) -> Result<(String, f64), Error> {
    let args = func.args()?;
    let [field, q]: [SqlExpr; 2] = args.try_into().map_err(|v: Vec<SqlExpr>| {
        Error::Invalid(format!("{name}: expected 2 arguments, got {}", v.len()))
    })?;
    Ok((
        field.as_ident().ok_or_else(|| {
            Error::Invalid(format!("{name}: first argument must be a column name"))
        })?,
        Value::from_sql(q)
            .ok()
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)))
            .filter(|q| (0.0..=1.0).contains(q))
            .ok_or_else(|| {
                Error::Invalid(format!(
                    "{name}: second argument must be a number in [0, 1]"
                ))
            })?,
    ))
}
