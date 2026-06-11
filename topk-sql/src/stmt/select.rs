use sqlparser::ast::{
    Expr as SqlExpr, FunctionArg, FunctionArgExpr, FunctionArguments, GroupByExpr,
    Query as SqlQuery, SelectItem, SetExpr, TableFactor, Value as SqlValue,
};
use topk_rs::proto::v1::data::stage::{filter_stage::FilterExpr, select_stage::SelectExpr};
use topk_rs::proto::v1::data::{LogicalExpr, Query, Stage};

use crate::sql_invalid;
use crate::stmt::info;
use crate::{Error, FromSql, SqlExprExt, Table, sql_unsupported, stmt::Statement};

impl TryFrom<SqlQuery> for Statement {
    type Error = Error;

    fn try_from(query: SqlQuery) -> Result<Statement, Error> {
        sql_unsupported!(query.with.is_some(), "WITH (common table expressions)");
        sql_unsupported!(!query.locks.is_empty(), "FOR UPDATE / FOR SHARE");
        sql_unsupported!(query.fetch.is_some(), "FETCH FIRST … ROWS ONLY");
        sql_unsupported!(query.offset.is_some(), "OFFSET");

        let mut select = match *query.body {
            SetExpr::Select(select) => match info::try_from_select(&select) {
                Some(result) => return result,
                None => select,
            },
            SetExpr::Query(_) => sql_unsupported!("subqueries"),
            SetExpr::SetOperation { .. } => {
                sql_unsupported!("SELECT ... UNION/INTERSECT/EXCEPT ...")
            }
            SetExpr::Values(_) => sql_unsupported!("SELECT ... VALUES ..."),
            SetExpr::Insert(_) => sql_unsupported!("SELECT ... INSERT ..."),
            SetExpr::Update(_) => sql_unsupported!("SELECT ... UPDATE ..."),
            SetExpr::Table(_) => sql_unsupported!("SELECT ... TABLE ..."),
        };

        sql_unsupported!(select.distinct.is_some(), "SELECT DISTINCT");
        sql_unsupported!(
            !matches!(select.group_by, GroupByExpr::Expressions(ref e, _) if e.is_empty()),
            "GROUP BY"
        );
        sql_unsupported!(select.having.is_some(), "HAVING");
        sql_unsupported!(!select.lateral_views.is_empty(), "LATERAL views");
        sql_unsupported!(select.named_window.iter().next().is_some(), "WINDOW");
        sql_unsupported!(select.into.is_some(), "SELECT INTO");

        let mut stages = Vec::new();

        // parse table name
        sql_invalid!(select.from.is_empty(), "SELECT requires a FROM clause");
        sql_unsupported!(select.from.len() != 1, "multiple tables in FROM");
        let first = select.from.swap_remove(0);
        sql_unsupported!(!first.joins.is_empty(), "JOIN");

        let table = match first.relation {
            TableFactor::Table { name, args, .. } => {
                sql_unsupported!(args.is_some(), "table-valued function in FROM");
                Table::new(name)?
            }
            other => sql_unsupported!("FROM clause: {other:?}"),
        };

        // parse WHERE
        if let Some(where_clause) = select.selection {
            stages.push(Stage::filter(FilterExpr::from_sql(where_clause)?));
        }

        let count_stage = if select.projection.len() == 1 {
            let expr_ref = match &select.projection[0] {
                SelectItem::UnnamedExpr(e) => Some(e),
                SelectItem::ExprWithAlias { expr: e, .. } => Some(e),
                _ => None,
            };
            match expr_ref {
                Some(SqlExpr::Function(func))
                    if func.name.0.len() == 1
                        && func.name.0[0].value.eq_ignore_ascii_case("count") =>
                {
                    let is_count_star = match &func.args {
                        FunctionArguments::List(list) if list.duplicate_treatment.is_none() => {
                            matches!(
                                list.args.as_slice(),
                                [FunctionArg::Unnamed(FunctionArgExpr::Wildcard)]
                            )
                        }
                        _ => false,
                    };

                    sql_unsupported!(
                        !is_count_star,
                        "only COUNT(*) is supported; COUNT(expr) and DISTINCT are not"
                    );

                    Some(Stage::count())
                }
                _ => None,
            }
        } else {
            for item in &select.projection {
                let expr = match item {
                    SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => e,
                    _ => continue,
                };
                if let SqlExpr::Function(func) = expr {
                    if func.name.0.len() == 1 && func.name.0[0].value.eq_ignore_ascii_case("count")
                    {
                        sql_unsupported!("COUNT(*) cannot be combined with other columns");
                    }
                }
            }
            None
        };

        if let Some(stage) = count_stage {
            sql_unsupported!(query.order_by.is_some(), "SELECT COUNT(*) ... ORDER BY ...");
            sql_unsupported!(query.limit.is_some(), "SELECT COUNT(*) ... LIMIT ...");

            stages.push(stage);
        } else {
            let mut projection = Vec::with_capacity(select.projection.len());
            for item in select.projection {
                match item {
                    SelectItem::UnnamedExpr(e) => {
                        projection.push((projection_alias(&e)?, SelectExpr::from_sql(e)?));
                    }
                    SelectItem::ExprWithAlias { expr: e, alias } => {
                        projection.push((alias.value, SelectExpr::from_sql(e)?));
                    }
                    SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(..) => {
                        sql_unsupported!("SELECT *");
                    }
                }
            }
            stages.push(Stage::select(projection));

            let sort = query
                .order_by
                .map(|mut order_by| match order_by.exprs.len() {
                    0 => Result::<_, Error>::Ok(None),
                    1 => {
                        let entry = order_by.exprs.pop().unwrap();

                        sql_unsupported!(
                            entry.nulls_first.is_some(),
                            "ORDER BY … NULLS FIRST/LAST"
                        );

                        sql_unsupported!(
                            matches!(&entry.expr, SqlExpr::Value(SqlValue::Number(_, _))),
                            "ORDER BY with ordinal position is not supported"
                        );
                        let converted = LogicalExpr::from_sql(entry.expr)?;
                        let asc = entry.asc.unwrap_or(true);
                        Ok(Some((converted, asc)))
                    }
                    _ => sql_unsupported!("ORDER BY with multiple keys is not supported"),
                })
                .transpose()?
                .flatten();

            let limit = match query.limit {
                None => None,
                Some(ref expr) => Some(expr.as_u64().ok_or_else(|| {
                    Error::Invalid("LIMIT must be a positive integer".to_string())
                })?),
            };

            match (sort, limit) {
                (Some((expr, asc)), Some(k)) => {
                    stages.push(Stage::sort(expr, asc));
                    stages.push(Stage::limit(k));
                }
                (Some(_), None) => sql_invalid!("ORDER BY without LIMIT is not supported"),
                (None, Some(k)) => stages.push(Stage::limit(k)),
                (None, None) => {}
            }
        }

        Ok(Statement::Select {
            table,
            query: Query { stages },
        })
    }
}

fn projection_alias(expr: &SqlExpr) -> Result<String, Error> {
    if let Some(name) = expr.as_ident() {
        return Ok(name);
    }

    match expr {
        SqlExpr::Function(f) => f
            .name
            .0
            .last()
            .map(|i| i.value.clone())
            .ok_or_else(|| Error::Invalid("function with no name".to_string())),
        SqlExpr::Cast { expr, .. } => projection_alias(expr),
        _ => Err(Error::Invalid(
            "expression in SELECT list requires an AS alias".to_string(),
        )),
    }
}
