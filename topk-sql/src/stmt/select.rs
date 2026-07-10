use std::collections::HashMap;
use std::ops::ControlFlow;

use sqlparser::ast::{
    Expr as SqlExpr, Function as SqlFunction, FunctionArg, FunctionArgExpr, GroupByExpr, Ident,
    LimitClause, ObjectNamePart, OrderByKind, Query as SqlQuery, SetExpr, TableFactor,
    Value as SqlValue, visit_expressions_mut,
};
use topk_rs::proto::v1::data::stage::sort_stage::SortOrder;
use topk_rs::proto::v1::data::stage::{filter_stage::FilterExpr, select_stage::SelectExpr};
use topk_rs::proto::v1::data::{AggregateExpr, LogicalExpr, Query, Stage};

use crate::{
    Error, FromSql, SelectItemExt, SqlExprExt, SqlFunctionExt, Table, sql_invalid, sql_unsupported,
    stmt::Statement,
};

fn is_aggregate_fn(func: &SqlFunction) -> bool {
    matches!(
        func.name().to_ascii_lowercase().as_str(),
        "count" | "sum" | "min" | "max" | "avg"
    )
}

/// Dedup key for reusing a HAVING aggregate that's already in the SELECT list. Only the
/// name is case-normalized — every other field must match exactly, so a call with an
/// unsupported modifier (FILTER, OVER, ...) is never silently treated as identical.
fn agg_signature(func: &SqlFunction) -> SqlFunction {
    let mut normalized = func.clone();
    for part in normalized.name.0.iter_mut() {
        if let ObjectNamePart::Identifier(ident) = part {
            ident.value = ident.value.to_ascii_lowercase();
        }
    }
    normalized
}

/// Fresh field name for a HAVING-only aggregate, guaranteed not to collide with any
/// user-chosen key or aggregate name.
fn fresh_internal_name(keys: &[(String, LogicalExpr)], aggs: &[(String, AggregateExpr)]) -> String {
    (0..)
        .map(|i| format!("topk_having_{i}"))
        .find(|name| {
            !keys.iter().any(|(n, _)| n == name) && !aggs.iter().any(|(n, _)| n == name)
        })
        .expect("infinite iterator always yields an unused name")
}

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

        let group_by_exprs: Vec<SqlExpr> = match select.group_by {
            GroupByExpr::Expressions(exprs, modifiers) if modifiers.is_empty() => exprs,
            GroupByExpr::Expressions(_, _) => {
                sql_unsupported!("GROUP BY ... WITH ROLLUP/CUBE/TOTALS/GROUPING SETS")
            }
            GroupByExpr::All(_) => sql_unsupported!("GROUP BY ALL"),
        };

        sql_invalid!(
            select.having.is_some() && group_by_exprs.is_empty(),
            "HAVING requires a GROUP BY clause"
        );
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

        let mut post_group_projection = None;

        if group_by_exprs.is_empty() {
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
                        sql_unsupported!(
                            query.order_by.is_some(),
                            "SELECT COUNT(*) ... ORDER BY ..."
                        );
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
        } else {
            let mut alias_map: HashMap<String, SqlExpr> = HashMap::new();
            for item in &select.projection {
                if let Some(expr) = item.expr() {
                    let is_agg = matches!(expr, SqlExpr::Function(f) if is_aggregate_fn(f));
                    // Computed aliases become GROUP BY keys; plain renames are validated below.
                    if !is_agg && expr.as_ident().is_none() {
                        if let Ok(name) = item.projection_name() {
                            alias_map.insert(name, expr.clone());
                        }
                    }
                }
            }

            let mut group_keys = Vec::with_capacity(group_by_exprs.len());
            for key_expr in group_by_exprs {
                let name = key_expr.as_ident().ok_or_else(|| {
                    Error::Unsupported(
                        "GROUP BY key must be a column name or a SELECT-list alias — alias \
                         computed expressions with AS in the SELECT list and GROUP BY the alias"
                            .to_string(),
                    )
                })?;
                let source = alias_map.remove(&name).unwrap_or_else(|| key_expr.clone());
                let logical = LogicalExpr::from_sql(source)?;
                group_keys.push((name, logical));
            }

            let mut keys = group_keys.clone();
            let mut aggs = Vec::new();
            let mut agg_sigs: HashMap<SqlFunction, String> = HashMap::new();
            let mut projection = Vec::with_capacity(select.projection.len());
            for item in select.projection {
                sql_unsupported!(item.is_wildcard(), "SELECT * with GROUP BY");
                let expr = item
                    .expr()
                    .expect("non-wildcard select item has an expression")
                    .clone();
                let out_name = item.projection_name()?;

                match &expr {
                    SqlExpr::Function(func) if is_aggregate_fn(func) => {
                        agg_sigs.insert(agg_signature(func), out_name.clone());
                        aggs.push((out_name.clone(), AggregateExpr::from_sql(func.clone())?));
                        projection.push((
                            out_name.clone(),
                            SelectExpr::logical(LogicalExpr::field(out_name)),
                        ));
                    }
                    _ => {
                        let logical = LogicalExpr::from_sql(expr)?;
                        sql_unsupported!(
                            !group_keys.iter().any(|(_, key)| key == &logical),
                            "`{out_name}` in a GROUP BY query must be a group key or an \
                             aggregate function call (COUNT/SUM/MIN/MAX/AVG)"
                        );
                        if !keys.iter().any(|(name, _)| name == &out_name) {
                            keys.push((out_name.clone(), logical));
                        }
                        projection.push((
                            out_name.clone(),
                            SelectExpr::logical(LogicalExpr::field(out_name)),
                        ));
                    }
                }
            }

            sql_unsupported!(
                aggs.is_empty(),
                "GROUP BY queries require at least one aggregate function"
            );

            // HAVING can call an aggregate directly, not just its alias. Rewrite each call
            // to a field reference, reusing the SELECT list's aggregate if identical, else
            // adding a new one to `aggs`.
            let having_filter = match select.having.take() {
                Some(mut having) => {
                    let result = visit_expressions_mut(&mut having, |expr| -> ControlFlow<Error> {
                        if let SqlExpr::Function(func) = expr {
                            if is_aggregate_fn(func) {
                                let sig = agg_signature(func);
                                let name = match agg_sigs.get(&sig) {
                                    Some(existing) => existing.clone(),
                                    None => {
                                        let agg = match AggregateExpr::from_sql(func.clone()) {
                                            Ok(agg) => agg,
                                            Err(e) => return ControlFlow::Break(e),
                                        };
                                        let synth = fresh_internal_name(&keys, &aggs);
                                        aggs.push((synth.clone(), agg));
                                        agg_sigs.insert(sig, synth.clone());
                                        synth
                                    }
                                };
                                *expr = SqlExpr::Identifier(Ident::new(name));
                            }
                        }
                        ControlFlow::Continue(())
                    });
                    if let ControlFlow::Break(e) = result {
                        return Err(e);
                    }
                    Some(LogicalExpr::from_sql(having)?)
                }
                None => None,
            };

            stages.push(Stage::group_by(keys, aggs));

            if let Some(having) = having_filter {
                stages.push(Stage::filter(having));
            }

            post_group_projection = Some(projection);
        }

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

        let (limit, offset) = match query.limit_clause {
            Some(LimitClause::OffsetCommaLimit { .. }) => unreachable!(
                "postgres dialect does not support `LIMIT offset, limit` should be rejected upstream"
            ),
            Some(LimitClause::LimitOffset { limit, offset, .. }) => {
                let limit = limit
                    .map(|ref expr| {
                        expr.as_u64().ok_or_else(|| {
                            Error::Invalid("LIMIT must be a positive integer".to_string())
                        })
                    })
                    .transpose()?;
                let offset = offset
                    .map(|o| {
                        o.value.as_u64().ok_or_else(|| {
                            Error::Invalid("OFFSET must be a positive integer".to_string())
                        })
                    })
                    .transpose()?;
                (limit, offset)
            }
            None => (None, None),
        };

        match (sort, limit) {
            (Some((expr, asc)), Some(k)) => {
                stages.push(Stage::sort((
                    expr,
                    asc.then_some(SortOrder::Asc).unwrap_or(SortOrder::Desc),
                )));
                stages.push(Stage::limit(k));
            }
            (Some(_), None) => sql_invalid!("ORDER BY without LIMIT is not supported"),
            (None, Some(k)) => stages.push(Stage::limit(k)),
            (None, None) => {
                if offset.is_some() {
                    sql_invalid!("OFFSET without LIMIT is not supported")
                }
            }
        }

        if let Some(off) = offset {
            stages.push(Stage::offset(off));
        }

        if let Some(projection) = post_group_projection {
            stages.push(Stage::select(projection));
        }

        Ok(Statement::Select {
            table,
            query: Query { stages },
        })
    }
}
