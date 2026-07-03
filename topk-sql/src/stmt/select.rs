use sqlparser::ast::{
    Expr as SqlExpr, FunctionArg, FunctionArgExpr, GroupByExpr, LimitClause, OrderByKind,
    Query as SqlQuery, SetExpr, TableFactor, Value as SqlValue,
};
use topk_rs::proto::v1::data::stage::{filter_stage::FilterExpr, select_stage::SelectExpr};
use topk_rs::proto::v1::data::{LogicalExpr, Query, Stage};

use crate::{
    Error, FromSql, SelectItemExt, SqlExprExt, SqlFunctionExt, Table, sql_invalid, sql_unsupported,
    stmt::Statement,
};

impl TryFrom<SqlQuery> for Statement {
    type Error = Error;

    fn try_from(query: SqlQuery) -> Result<Statement, Error> {
        sql_unsupported!(query.with.is_some(), "WITH (common table expressions)");
        sql_unsupported!(!query.locks.is_empty(), "FOR UPDATE / FOR SHARE");
        sql_unsupported!(query.fetch.is_some(), "FETCH FIRST … ROWS ONLY");

        let mut select = match *query.body {
            SetExpr::Select(select) => select,
            SetExpr::Query(_) => sql_unsupported!("subqueries"),
            SetExpr::SetOperation { .. } => {
                sql_unsupported!("SELECT ... UNION/INTERSECT/EXCEPT ...")
            }
            SetExpr::Values(_) => sql_unsupported!("SELECT ... VALUES ..."),
            SetExpr::Insert(_) => sql_unsupported!("SELECT ... INSERT ..."),
            SetExpr::Update(_) => sql_unsupported!("SELECT ... UPDATE ..."),
            SetExpr::Table(_) => sql_unsupported!("SELECT ... TABLE ..."),
            SetExpr::Delete(_) => sql_unsupported!("SELECT ... DELETE ..."),
            SetExpr::Merge(_) => sql_unsupported!("SELECT ... MERGE ..."),
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
            stages.extend(
                Vec::<FilterExpr>::from_sql(where_clause)?
                    .into_iter()
                    .map(Stage::filter),
            );
        }

        if select.projection.len() == 1 {
            let item = &select.projection[0];
            if let Some(SqlExpr::Function(func)) = item.expr() {
                if func.is_count() {
                    sql_unsupported!(
                        !func.matches_args(|args| {
                            matches!(args, [FunctionArg::Unnamed(FunctionArgExpr::Wildcard)])
                        }),
                        "only COUNT(*) is supported; COUNT(expr) and DISTINCT are not"
                    );
                    sql_unsupported!(query.order_by.is_some(), "SELECT COUNT(*) ... ORDER BY ...");
                    sql_unsupported!(
                        query.limit_clause.is_some(),
                        "SELECT COUNT(*) ... LIMIT ..."
                    );

                    stages.push(Stage::count());

                    return Ok(Statement::Count {
                        table,
                        query: Query { stages },
                        alias: item.column_name(),
                    });
                }
            }
        } else {
            for item in &select.projection {
                if let Some(SqlExpr::Function(func)) = item.expr() {
                    if func.is_count() {
                        sql_unsupported!("COUNT(*) cannot be combined with other columns");
                    }
                }
            }
        }

        let mut projection = Vec::with_capacity(select.projection.len());
        for item in select.projection {
            if item.is_wildcard() {
                sql_unsupported!("SELECT *");
            }
            let expr = item
                .expr()
                .expect("non-wildcard select item has an expression");
            projection.push((item.projection_name()?, SelectExpr::from_sql(expr.clone())?));
        }
        stages.push(Stage::select(projection));

        let sort = query
            .order_by
            .map(|order_by| match order_by.kind {
                OrderByKind::Expressions(ref exprs) if exprs.is_empty() => {
                    Result::<_, Error>::Ok(None)
                }
                OrderByKind::Expressions(mut exprs) if exprs.len() == 1 => {
                    let entry = exprs.pop().unwrap();

                    sql_unsupported!(
                        entry.options.nulls_first.is_some(),
                        "ORDER BY … NULLS FIRST/LAST"
                    );
                    sql_unsupported!(
                        matches!(&entry.expr, SqlExpr::Value(v) if matches!(v.value, SqlValue::Number(_, _))),
                        "ORDER BY with ordinal position is not supported"
                    );

                    let converted = LogicalExpr::from_sql(entry.expr)?;
                    let asc = entry.options.asc.unwrap_or(true);
                    Ok(Some((converted, asc)))
                }
                _ => sql_unsupported!("ORDER BY with multiple keys is not supported"),
            })
            .transpose()?
            .flatten();

        let limit =
            match query.limit_clause {
                Some(LimitClause::LimitOffset {
                    offset: Some(_), ..
                })
                | Some(LimitClause::OffsetCommaLimit { .. }) => sql_unsupported!("OFFSET"),
                Some(LimitClause::LimitOffset {
                    limit: Some(ref expr),
                    ..
                }) => Some(expr.as_u64().ok_or_else(|| {
                    Error::Invalid("LIMIT must be a positive integer".to_string())
                })?),
                Some(_) => sql_invalid!("LIMIT must be a positive integer"),
                None => None,
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

        Ok(Statement::Select {
            table,
            query: Query { stages },
        })
    }
}
