use sqlparser::ast::{DataType, Expr as SqlExpr, UnaryOperator, Value as SqlValue};

use topk_rs::proto::v1::data::Value;
use topk_rs::proto::v1::data::value::Value::{self as V};

use super::typed::ElemType;
use crate::{Error, FromSql, sql_unsupported};

impl FromSql<SqlExpr> for Value {
    fn from_sql(expr: SqlExpr) -> Result<Value, Error> {
        match expr {
            SqlExpr::Nested(inner) => Value::from_sql(*inner),
            SqlExpr::Value(v) => match v.value {
                SqlValue::Number(repr, _long) => parse_number(&repr),
                SqlValue::SingleQuotedString(s) | SqlValue::DoubleQuotedString(s) => {
                    Ok(Value::string(s))
                }
                SqlValue::Boolean(b) => Ok(Value::bool(b)),
                SqlValue::Null => Ok(Value::null()),
                other => Err(Error::InvalidLiteral(format!("{other:?}"))),
            },
            SqlExpr::UnaryOp {
                op: UnaryOperator::Minus,
                expr,
            } => match *expr {
                SqlExpr::Value(v) if matches!(v.value, SqlValue::Number(_, _)) => {
                    let repr = match v.value {
                        SqlValue::Number(repr, _) => repr,
                        _ => unreachable!(),
                    };
                    parse_number(&format!("-{repr}"))
                }
                SqlExpr::Nested(inner) => Value::from_sql(SqlExpr::UnaryOp {
                    op: UnaryOperator::Minus,
                    expr: inner,
                }),
                other => sql_unsupported!("expression as value: {other:?}"),
            },
            SqlExpr::Array(arr) => Value::from_sql(arr.elem),
            SqlExpr::Function(func) => Value::from_sql(func),
            SqlExpr::Cast {
                expr, data_type, ..
            } => {
                let s = match *expr {
                    SqlExpr::Value(v) => match v.value {
                        SqlValue::SingleQuotedString(s) | SqlValue::DoubleQuotedString(s) => s,
                        other => {
                            sql_unsupported!("cast source must be a string literal, got: {other:?}")
                        }
                    },
                    other => {
                        sql_unsupported!("cast source must be a string literal, got: {other:?}")
                    }
                };
                let type_name = match data_type {
                    DataType::Custom(ref name, _) => name.to_string().to_ascii_lowercase(),
                    other => return Err(Error::Unsupported(format!("cast to {other:?}"))),
                };
                parse_cast(&type_name, &s)
            }
            other => sql_unsupported!("expression as value: {other:?}"),
        }
    }
}

impl FromSql<Vec<SqlExpr>> for Value {
    fn from_sql(elements: Vec<SqlExpr>) -> Result<Value, Error> {
        if elements.is_empty() {
            return Err(Error::Invalid(
                "empty ARRAY[]: element type cannot be inferred".to_string(),
            ));
        }
        let values: Vec<Value> = elements
            .into_iter()
            .map(Value::from_sql)
            .collect::<Result<_, _>>()?;

        match values.first().and_then(|v| v.value.as_ref()) {
            Some(V::I64(_)) => collect_into(values, |v| match v {
                V::I64(n) => Some(n),
                _ => None,
            })
            .map(Value::list),
            Some(V::F64(_)) => collect_into(values, |v| match v {
                V::F64(n) => Some(n),
                V::I64(n) => Some(n as f64),
                _ => None,
            })
            .map(Value::list),
            Some(V::String(_)) => collect_into(values, |v| match v {
                V::String(s) => Some(s),
                _ => None,
            })
            .map(Value::list),
            _ => Err(Error::Invalid(
                "ARRAY[...]: unsupported element type".to_string(),
            )),
        }
    }
}

fn collect_into<T>(values: Vec<Value>, extract: impl Fn(V) -> Option<T>) -> Result<Vec<T>, Error> {
    values
        .into_iter()
        .map(|v| {
            v.value
                .and_then(&extract)
                .ok_or_else(|| Error::Invalid("ARRAY[...]: mixed element types".to_string()))
        })
        .collect()
}

pub fn parse_number(repr: &str) -> Result<Value, Error> {
    if repr.contains('.') || repr.contains('e') || repr.contains('E') {
        return repr
            .parse::<f64>()
            .map(Value::f64)
            .map_err(|e| Error::InvalidLiteral(format!("invalid float literal `{repr}`: {e}")));
    }

    if let Ok(n) = repr.parse::<i64>() {
        return Ok(Value::i64(n));
    }

    if let Ok(n) = repr.parse::<u64>() {
        return Ok(Value::u64(n));
    }

    repr.parse::<f64>()
        .map(Value::f64)
        .map_err(|e| Error::InvalidLiteral(format!("invalid integer literal `{repr}`: {e}")))
}

fn parse_cast(type_name: &str, s: &str) -> Result<Value, Error> {
    if let Some(elem_type) = ElemType::from_dense_type_name(type_name) {
        let floats: Vec<f64> = serde_json::from_str(s)?;
        return Ok(elem_type.from_floats(floats, type_name)?.into_list_value());
    }

    if let Some(elem_type) = ElemType::from_sparse_type_name(type_name) {
        let obj: serde_json::Map<String, serde_json::Value> = serde_json::from_str(s)?;
        let (indices, values) = if obj.contains_key("indices") || obj.contains_key("values") {
            // {"indices": [...], "values": [...]} format
            let indices = obj
                .get("indices")
                .and_then(|v| v.as_array())
                .ok_or_else(|| Error::Invalid(format!("{type_name}: expected \"indices\" array")))?
                .iter()
                .map(|v| {
                    v.as_u64()
                        .and_then(|n| u32::try_from(n).ok())
                        .ok_or_else(|| Error::Invalid(format!("{type_name}: index must be a u32")))
                })
                .collect::<Result<Vec<u32>, _>>()?;
            let values = obj
                .get("values")
                .and_then(|v| v.as_array())
                .ok_or_else(|| Error::Invalid(format!("{type_name}: expected \"values\" array")))?
                .iter()
                .map(|v| {
                    v.as_f64().ok_or_else(|| {
                        Error::Invalid(format!("{type_name}: value must be a number"))
                    })
                })
                .collect::<Result<Vec<f64>, _>>()?;
            if indices.len() != values.len() {
                return Err(Error::Invalid(format!(
                    "{type_name}: indices length ({}) must match values length ({})",
                    indices.len(),
                    values.len()
                )));
            }
            (indices, values)
        } else {
            // {"0": 1.0, "2": 0.5} format
            let mut entries: Vec<(u32, f64)> = obj
                .iter()
                .map(|(k, v)| -> Result<(u32, f64), Error> {
                    let idx = k.parse::<u32>().map_err(|_| {
                        Error::Invalid(format!(
                            "{type_name}: index must be a non-negative integer: {k}"
                        ))
                    })?;
                    let val = v.as_f64().ok_or_else(|| {
                        Error::Invalid(format!("{type_name}: value must be a number"))
                    })?;
                    Ok((idx, val))
                })
                .collect::<Result<_, _>>()?;
            entries.sort_by_key(|(i, _)| *i);
            entries.into_iter().unzip()
        };
        return elem_type
            .from_floats(values, type_name)?
            .into_sparse_value(indices, type_name);
    }

    if let Some(elem_type) = ElemType::from_matrix_type_name(type_name) {
        let rows: Vec<Vec<f64>> = serde_json::from_str(s)?;
        if rows.is_empty() {
            return Err(Error::Invalid(format!(
                "{type_name}: must have at least one row"
            )));
        }
        let num_cols = rows[0].len() as u32;
        for (i, row) in rows.iter().enumerate() {
            if row.len() as u32 != num_cols {
                return Err(Error::Invalid(format!(
                    "{type_name}: all rows must have the same length (row {i} has {} vs {num_cols})",
                    row.len()
                )));
            }
        }
        let flat: Vec<f64> = rows.into_iter().flatten().collect();
        return elem_type
            .from_floats(flat, type_name)?
            .into_matrix_value(num_cols, type_name);
    }

    Err(Error::Unsupported(format!("cast to {type_name}")))
}
