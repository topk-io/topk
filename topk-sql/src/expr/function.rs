use sqlparser::ast::{Expr as SqlExpr, Function as SqlFunction};

use topk_rs::proto::v1::data::{
    FunctionExpr, LogicalExpr, TextExpr, Value, list, text_expr::Term, value,
};

use super::typed::{ElemType, TypedValues, coerce_i64s};
use crate::expr::Expr;
use crate::expr::regexp;
use crate::ext::{SqlExprExt, SqlFunctionExt};
use crate::{Error, FromSql, sql_invalid, sql_unsupported};

impl TryFrom<SqlFunction> for Expr {
    type Error = Error;

    fn try_from(func: SqlFunction) -> Result<Self, Self::Error> {
        let name = func.name();
        let key = name.to_ascii_lowercase();
        let args = func.args()?;

        Ok(match key.as_str() {
            "abs" => Self::Logical(unary(args, &name)?.abs()),
            "ln" => Self::Logical(unary(args, &name)?.ln()),
            "exp" => Self::Logical(unary(args, &name)?.exp()),
            "sqrt" => Self::Logical(unary(args, &name)?.sqrt()),
            "square" => Self::Logical(unary(args, &name)?.square()),
            "coalesce" => {
                let (left, right) = binary(args, &name)?;
                Self::Logical(left.coalesce(right))
            }
            "least" => {
                let (left, right) = binary(args, &name)?;
                Self::Logical(left.min(right))
            }
            "greatest" => {
                let (left, right) = binary(args, &name)?;
                Self::Logical(left.max(right))
            }
            "match_all" => {
                let [left, right]: [SqlExpr; 2] = exact(args, &name)?;
                Self::Logical(LogicalExpr::from_sql(left)?.match_all(LogicalExpr::from_sql(right)?))
            }
            "match_any" => {
                let [left, right]: [SqlExpr; 2] = exact(args, &name)?;
                Self::Logical(LogicalExpr::from_sql(left)?.match_any(LogicalExpr::from_sql(right)?))
            }
            "regexp_like" => {
                let (string, pattern, pg_flags) = match args.len() {
                    2 => {
                        let [string, pattern]: [SqlExpr; 2] = exact(args, &name)?;
                        (string, pattern, None)
                    }
                    3 => {
                        let [string, pattern, flags]: [SqlExpr; 3] = exact(args, &name)?;
                        (string, pattern, Some(flags))
                    }
                    n => sql_invalid!("{name}: expected 2..=3 args, got {n}"),
                };
                let pattern = pattern.as_string().ok_or_else(|| {
                    Error::Invalid(format!("{name}: pattern must be a string literal"))
                })?;
                let pg_flags = match pg_flags {
                    Some(flags) => flags.as_string().ok_or_else(|| {
                        Error::Invalid(format!("{name}: flags must be a string literal"))
                    })?,
                    None => String::new(),
                };
                let (pattern, flags) = regexp::translate(&pattern, &pg_flags)?;
                Self::Logical(LogicalExpr::from_sql(string)?.regexp_match(pattern, flags))
            }
            "match" => Self::Text(text_match(args, &name)?),
            "match_tokens" => Self::Text(text_match_tokens(args, &name)?),
            "should" => Self::Text(text_should(args, &name)?),
            "boost" => {
                let [score, cond, factor]: [SqlExpr; 3] = exact(args, &name)?;
                let score = LogicalExpr::from_sql(score)?;
                let cond = LogicalExpr::from_sql(cond)?;
                let factor = Value::from_sql(factor)?;
                Self::Logical(score.mul(cond.choose(LogicalExpr::literal(factor), Value::i64(1))))
            }
            "binary_vector" => Self::Literal(list_ctor(args, &name, ElemType::U8Vector)?),
            "f16_vector" => Self::Literal(list_ctor(args, &name, ElemType::F16Vector)?),
            "f32_vector" => Self::Literal(list_ctor(args, &name, ElemType::F32Vector)?),
            "f64_vector" => Self::Literal(list_ctor(args, &name, ElemType::F64Vector)?),
            "f8_vector" => Self::Literal(list_ctor(args, &name, ElemType::F8Vector)?),
            "u8_vector" => Self::Literal(list_ctor(args, &name, ElemType::U8Vector)?),
            "u32_vector" => Self::Literal(list_ctor(args, &name, ElemType::U32Vector)?),
            "u64_vector" => Self::Literal(list_ctor(args, &name, ElemType::U64Vector)?),
            "i8_vector" => Self::Literal(list_ctor(args, &name, ElemType::I8Vector)?),
            "i32_vector" => Self::Literal(list_ctor(args, &name, ElemType::I32Vector)?),
            "i64_vector" => Self::Literal(list_ctor(args, &name, ElemType::I64Vector)?),
            "f16_sparse_vector" => Self::Literal(sparse_ctor(args, &name, ElemType::F16Vector)?),
            "f32_sparse_vector" => Self::Literal(sparse_ctor(args, &name, ElemType::F32Vector)?),
            "f8_sparse_vector" => Self::Literal(sparse_ctor(args, &name, ElemType::F8Vector)?),
            "u8_sparse_vector" => Self::Literal(sparse_ctor(args, &name, ElemType::U8Vector)?),
            "i8_sparse_vector" => Self::Literal(sparse_ctor(args, &name, ElemType::I8Vector)?),
            "f16_matrix" => Self::Literal(matrix_ctor(args, &name, ElemType::F16Vector)?),
            "f32_matrix" => Self::Literal(matrix_ctor(args, &name, ElemType::F32Vector)?),
            "f8_matrix" => Self::Literal(matrix_ctor(args, &name, ElemType::F8Vector)?),
            "u8_matrix" => Self::Literal(matrix_ctor(args, &name, ElemType::U8Vector)?),
            "i8_matrix" => Self::Literal(matrix_ctor(args, &name, ElemType::I8Vector)?),
            "bytes" => {
                let [arg]: [SqlExpr; 1] = exact(args, &name)?;
                let s = arg.as_string().ok_or_else(|| {
                    Error::Invalid(format!("{name}: argument must be a hex string"))
                })?;
                let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
                if !cleaned.len().is_multiple_of(2) {
                    return Err(Error::Invalid("bytes: hex string has odd length".into()));
                }
                let bytes = (0..cleaned.len())
                    .step_by(2)
                    .map(|i| {
                        u8::from_str_radix(&cleaned[i..i + 2], 16).map_err(|e| {
                            Error::Invalid(format!(
                                "bytes: invalid hex `{}`: {e}",
                                &cleaned[i..i + 2]
                            ))
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Self::Literal(Value::bytes(bytes))
            }
            "struct" => {
                if !args.len().is_multiple_of(2) {
                    return Err(Error::Invalid(format!(
                        "{name}: expected (field, value) pairs"
                    )));
                }
                let mut fields: Vec<(String, Value)> = Vec::with_capacity(args.len() / 2);
                let mut it = args.into_iter();
                while let Some(k_expr) = it.next() {
                    let v_expr = it.next().unwrap();
                    let k = k_expr.as_string().ok_or_else(|| {
                        Error::Invalid(format!("{name}: field name must be a string literal"))
                    })?;
                    fields.push((k, Value::from_sql(v_expr)?));
                }
                Self::Literal(Value::r#struct(fields))
            }
            "contains" => {
                let [field, value]: [SqlExpr; 2] = exact(args, &name)?;
                let field = LogicalExpr::from_sql(field)?;
                let value = Value::from_sql(value)?;
                Self::Logical(field.contains(value))
            }
            "vector_distance" => {
                let (field, query, skip_refine) = match args.len() {
                    2 => {
                        let [field, query]: [SqlExpr; 2] = exact(args, &name)?;
                        (field, query, false)
                    }
                    3 => {
                        let [field, query, skip_refine]: [SqlExpr; 3] = exact(args, &name)?;
                        let skip_refine = skip_refine.as_bool().ok_or_else(|| {
                            Error::Invalid(format!("{name}: skip_refine must be a bool literal"))
                        })?;
                        (field, query, skip_refine)
                    }
                    n => sql_invalid!("{name}: expected 2..=3 args, got {n}"),
                };
                let field = field.as_ident().ok_or_else(|| {
                    Error::Invalid(format!("{name}: field must be an identifier"))
                })?;
                Self::Function(FunctionExpr::vector_distance(
                    field,
                    Value::from_sql(query)?,
                    skip_refine,
                ))
            }
            "multi_vector_distance" => {
                let (field, query, candidates) = match args.len() {
                    2 => {
                        let [field, query]: [SqlExpr; 2] = exact(args, &name)?;
                        (field, query, None)
                    }
                    3 => {
                        let [field, query, candidates]: [SqlExpr; 3] = exact(args, &name)?;
                        let candidates = candidates
                            .as_i64()
                            .and_then(|n| u32::try_from(n).ok())
                            .ok_or_else(|| {
                                Error::Invalid(format!("{name}: candidates must be a u32 literal"))
                            })?;
                        (field, query, Some(candidates))
                    }
                    n => sql_invalid!("{name}: expected 2..=3 args, got {n}"),
                };
                let field = field.as_ident().ok_or_else(|| {
                    Error::Invalid(format!("{name}: field must be an identifier"))
                })?;
                Self::Function(FunctionExpr::multi_vector_distance(
                    field,
                    Value::from_sql(query)?,
                    candidates,
                ))
            }
            "bm25_score" => match args.len() {
                0 => Self::Function(FunctionExpr::bm25_score(None, None)),
                2 => {
                    let [b, k1]: [SqlExpr; 2] = exact(args, &name)?;
                    Self::Function(FunctionExpr::bm25_score(
                        Some(f32_literal(b)?),
                        Some(f32_literal(k1)?),
                    ))
                }
                n => sql_invalid!("{name}: expected 0 or 2 args, got {n}"),
            },
            "semantic_similarity" => {
                let [field, query]: [SqlExpr; 2] = exact(args, &name)?;
                let query = query.as_string().ok_or_else(|| {
                    Error::Invalid(format!("{name}: query must be a string literal"))
                })?;
                let field = field.as_ident().ok_or_else(|| {
                    Error::Invalid(format!("{name}: field must be an identifier"))
                })?;
                Self::Function(FunctionExpr::semantic_similarity(field, query))
            }
            _ => return Err(Error::UnknownFunction(name)),
        })
    }
}

fn unary(args: Vec<SqlExpr>, sig: &str) -> Result<LogicalExpr, Error> {
    let [arg]: [SqlExpr; 1] = exact(args, sig)?;
    LogicalExpr::from_sql(arg)
}

fn binary(args: Vec<SqlExpr>, sig: &str) -> Result<(LogicalExpr, LogicalExpr), Error> {
    let [left, right]: [SqlExpr; 2] = exact(args, sig)?;
    Ok((LogicalExpr::from_sql(left)?, LogicalExpr::from_sql(right)?))
}

fn text_match(args: Vec<SqlExpr>, name: &str) -> Result<TextExpr, Error> {
    let (query, field, weight, all) = match args.len() {
        1 => {
            let [query]: [SqlExpr; 1] = exact(args, name)?;
            (query, None, 1.0, false)
        }
        2 => {
            let [query, field]: [SqlExpr; 2] = exact(args, name)?;
            (query, optional_field(field, name)?, 1.0, false)
        }
        3 => {
            let [query, field, weight]: [SqlExpr; 3] = exact(args, name)?;
            (
                query,
                optional_field(field, name)?,
                f32_literal(weight)?,
                false,
            )
        }
        4 => {
            let [query, field, weight, all]: [SqlExpr; 4] = exact(args, name)?;
            let all = all
                .as_bool()
                .ok_or_else(|| Error::Invalid(format!("{name}: all must be a bool literal")))?;
            (
                query,
                optional_field(field, name)?,
                f32_literal(weight)?,
                all,
            )
        }
        n => sql_invalid!("{name}: expected 1..=4 args, got {n}"),
    };

    let token = query
        .as_string()
        .ok_or_else(|| Error::Invalid(format!("{name}: query must be a string literal")))?;

    Ok(TextExpr::terms(
        all,
        vec![Term {
            token,
            field,
            weight,
        }],
    ))
}

fn text_should(args: Vec<SqlExpr>, name: &str) -> Result<TextExpr, Error> {
    let (query, field, weight) = match args.len() {
        1 => {
            let [query]: [SqlExpr; 1] = exact(args, name)?;
            (query, None, 1.0)
        }
        2 => {
            let [query, field]: [SqlExpr; 2] = exact(args, name)?;
            (query, optional_field(field, name)?, 1.0)
        }
        3 => {
            let [query, field, weight]: [SqlExpr; 3] = exact(args, name)?;
            (query, optional_field(field, name)?, f32_literal(weight)?)
        }
        n => sql_invalid!("{name}: expected 1..=3 args, got {n}"),
    };

    let token = query
        .as_string()
        .ok_or_else(|| Error::Invalid(format!("{name}: query must be a string literal")))?;

    Ok(TextExpr::should(vec![Term {
        token,
        field,
        weight,
    }]))
}

fn text_match_tokens(args: Vec<SqlExpr>, name: &str) -> Result<TextExpr, Error> {
    let (tokens, field, all) = match args.len() {
        1 => {
            let [tokens]: [SqlExpr; 1] = exact(args, name)?;
            (tokens, None, false)
        }
        2 => {
            let [tokens, field]: [SqlExpr; 2] = exact(args, name)?;
            (tokens, optional_field(field, name)?, false)
        }
        3 => {
            let [tokens, field, all]: [SqlExpr; 3] = exact(args, name)?;
            let all = all
                .as_bool()
                .ok_or_else(|| Error::Invalid(format!("{name}: all must be a bool literal")))?;
            (tokens, optional_field(field, name)?, all)
        }
        n => sql_invalid!("{name}: expected 1..=3 args, got {n}"),
    };

    let value = Value::from_sql(tokens)?;
    let tokens = match value.value {
        Some(value::Value::List(values)) => match values.values {
            Some(list::Values::String(tokens)) => tokens.values,
            _ => {
                return Err(Error::Invalid(format!(
                    "{name}: tokens must be an array of strings"
                )));
            }
        },
        _ => {
            return Err(Error::Invalid(format!(
                "{name}: tokens must be an array of strings"
            )));
        }
    };

    Ok(TextExpr::terms(
        all,
        tokens
            .into_iter()
            .map(|token| Term {
                token,
                field: field.clone(),
                weight: 1.0,
            })
            .collect(),
    ))
}

fn optional_field(expr: SqlExpr, name: &str) -> Result<Option<String>, Error> {
    if let Ok(value) = Value::from_sql(expr.clone()) {
        if value.as_null().is_some() {
            return Ok(None);
        }
        if let Some(field) = value.as_string() {
            return Ok(Some(field.to_string()));
        }
    }

    expr.as_ident().map(Some).ok_or_else(|| {
        Error::Invalid(format!(
            "{name}: field must be an identifier, string, or NULL"
        ))
    })
}

fn f32_literal(expr: SqlExpr) -> Result<f32, Error> {
    let v = Value::from_sql(expr)?;
    v.as_f64()
        .or_else(|| v.as_i64().map(|n| n as f64))
        .map(|x| x as f32)
        .ok_or_else(|| Error::Invalid("expected a numeric literal".to_string()))
}

impl FromSql<SqlFunction> for Value {
    fn from_sql(func: SqlFunction) -> Result<Value, Error> {
        let name = func.name();
        match Expr::try_from(func)? {
            Expr::Literal(v) => Ok(v),
            Expr::Logical(_) | Expr::Text(_) | Expr::Function(_) => {
                sql_unsupported!("`{name}` does not produce a Value")
            }
        }
    }
}

impl FromSql<SqlFunction> for LogicalExpr {
    fn from_sql(func: SqlFunction) -> Result<LogicalExpr, Error> {
        let name = func.name();
        match Expr::try_from(func)? {
            Expr::Logical(expr) => Ok(expr),
            Expr::Literal(value) => Ok(LogicalExpr::literal(value)),
            Expr::Text(_) => sql_unsupported!(
                "`{name}` is a text filter function — only valid in WHERE (e.g. \
                 `WHERE {name}('query', field)`)"
            ),
            Expr::Function(_) => sql_unsupported!(
                "`{name}` is a search function — only valid at the top of a SELECT projection \
                 item (e.g. `SELECT {name}(…) AS s FROM c ORDER BY s LIMIT k`)"
            ),
        }
    }
}

// =============================================================================
// Ctor-args helpers.
// =============================================================================

fn list_ctor(args: Vec<SqlExpr>, name: &str, elem_type: ElemType) -> Result<Value, Error> {
    Ok(parse_typed_values(single_array(args, name)?, name, elem_type)?.into_list_value())
}

fn sparse_ctor(args: Vec<SqlExpr>, name: &str, elem_type: ElemType) -> Result<Value, Error> {
    let [idx, val]: [SqlExpr; 2] = exact(args, name)?;
    let idx = array_elems(idx, name)?;
    let val = array_elems(val, name)?;
    if idx.len() != val.len() {
        return Err(Error::Invalid(format!(
            "{name}: index and value arrays must have equal length"
        )));
    }
    parse_typed_values(val, name, elem_type)?.into_sparse_value(parse_ints(idx, name, "u32")?, name)
}

fn matrix_ctor(args: Vec<SqlExpr>, name: &str, elem_type: ElemType) -> Result<Value, Error> {
    let outer = single_array(args, name)?;
    if outer.is_empty() {
        return Err(Error::Invalid(format!(
            "{name}: must have at least one row"
        )));
    }
    let mut num_cols: Option<usize> = None;
    let mut values = Vec::new();
    for row in outer {
        let row_elems = match row {
            SqlExpr::Array(arr) => arr.elem,
            other => {
                return Err(Error::Invalid(format!(
                    "{name}: expected ARRAY of ARRAY, got {other:?}"
                )));
            }
        };
        match num_cols {
            None => num_cols = Some(row_elems.len()),
            Some(n) if n == row_elems.len() => {}
            Some(n) => {
                return Err(Error::Invalid(format!(
                    "{name}: all rows must have the same length (got {} after {n})",
                    row_elems.len()
                )));
            }
        }
        values.extend(row_elems);
    }
    let cols = num_cols.unwrap() as u32;
    let flat = parse_typed_values(values, name, elem_type)?;
    flat.into_matrix_value(cols, name)
}

fn parse_typed_values(
    elements: Vec<SqlExpr>,
    sig: &str,
    elem_type: ElemType,
) -> Result<TypedValues, Error> {
    match elem_type {
        ElemType::F16Vector | ElemType::F32Vector | ElemType::F64Vector | ElemType::F8Vector => {
            elem_type.from_floats(parse_floats(elements, sig)?, sig)
        }
        ElemType::U8Vector
        | ElemType::U32Vector
        | ElemType::U64Vector
        | ElemType::I8Vector
        | ElemType::I32Vector
        | ElemType::I64Vector => elem_type.from_i64s(parse_i64s(elements, sig)?, sig),
    }
}

fn single_array(args: Vec<SqlExpr>, sig: &str) -> Result<Vec<SqlExpr>, Error> {
    let [arr]: [SqlExpr; 1] = exact(args, sig)?;
    array_elems(arr, sig)
}

fn array_elems(expr: SqlExpr, sig: &str) -> Result<Vec<SqlExpr>, Error> {
    match expr {
        SqlExpr::Array(a) => Ok(a.elem),
        other => Err(Error::Invalid(format!(
            "{sig}: expected an ARRAY[…] literal, got {other:?}"
        ))),
    }
}

fn parse_floats(elements: Vec<SqlExpr>, sig: &str) -> Result<Vec<f64>, Error> {
    require_non_empty(&elements, sig)?;
    elements
        .into_iter()
        .map(|e| {
            let v = Value::from_sql(e)?;
            v.as_f64()
                .or_else(|| v.as_i64().map(|n| n as f64))
                .ok_or_else(|| Error::Invalid(format!("{sig}: expected numeric elements")))
        })
        .collect()
}

fn parse_ints<T: TryFrom<i64>>(
    elements: Vec<SqlExpr>,
    sig: &str,
    target: &str,
) -> Result<Vec<T>, Error> {
    coerce_i64s(parse_i64s(elements, sig)?, sig, target)
}

fn parse_i64s(elements: Vec<SqlExpr>, sig: &str) -> Result<Vec<i64>, Error> {
    require_non_empty(&elements, sig)?;
    elements
        .into_iter()
        .map(|e| {
            Value::from_sql(e)?
                .as_i64()
                .ok_or_else(|| Error::Invalid(format!("{sig}: expected integer elements")))
        })
        .collect()
}

fn require_non_empty(elements: &[SqlExpr], sig: &str) -> Result<(), Error> {
    if elements.is_empty() {
        Err(Error::Invalid(format!("{sig}: must be non-empty")))
    } else {
        Ok(())
    }
}

fn exact<const N: usize>(args: Vec<SqlExpr>, sig: &str) -> Result<[SqlExpr; N], Error> {
    args.try_into()
        .map_err(|v: Vec<_>| Error::Invalid(format!("{sig}: expected {N} args, got {}", v.len())))
}
