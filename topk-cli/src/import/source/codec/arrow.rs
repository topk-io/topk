use duckdb::arrow::array::{self, Array, ArrayRef, AsArray};
use duckdb::arrow::datatypes::{DataType, Float64Type, Int64Type, TimeUnit, UInt64Type};
use topk_rs::proto::v1::data::Value;

use crate::import::decode::floats;
use crate::import::error::Error;
use crate::import::spec::Type;

pub fn ty(input: &DataType) -> Type {
    match input {
        DataType::Boolean => Type::Bool,
        DataType::Int8
        | DataType::Int16
        | DataType::Int32
        | DataType::Int64
        | DataType::UInt8
        | DataType::UInt16
        | DataType::UInt32
        | DataType::UInt64 => Type::Int,
        DataType::Float16 | DataType::Float32 | DataType::Float64 => Type::Float,
        // A decimal wider than f64 can hold (~15 digits) keeps its exact value
        // only as text.
        DataType::Decimal128(p, _) | DataType::Decimal256(p, _) => {
            if *p > 15 {
                Type::Text
            } else {
                Type::Float
            }
        }
        DataType::Binary | DataType::LargeBinary | DataType::FixedSizeBinary(_) => Type::Bytes,
        DataType::List(f) | DataType::LargeList(f) | DataType::FixedSizeList(f, _) => {
            match ty(f.data_type()) {
                Type::Int => Type::IntList,
                Type::Float => Type::FloatList,
                _ => Type::TextList,
            }
        }
        DataType::Struct(_) => Type::Struct,
        // value() renders these to RFC 3339 / a bare date, which the Timestamp
        // coercion reads back to epoch millis.
        DataType::Timestamp(_, _) | DataType::Date32 | DataType::Date64 => Type::Timestamp,
        _ => Type::Text,
    }
}

pub fn value(array: &ArrayRef, row: usize) -> Result<Value, Error> {
    if array.is_null(row) {
        return Ok(Value::null());
    }
    macro_rules! get {
        ($ty:ty) => {
            array.as_any().downcast_ref::<$ty>().unwrap().value(row)
        };
    }
    macro_rules! decimal {
        ($ty:ty) => {
            Value::string(
                array
                    .as_any()
                    .downcast_ref::<$ty>()
                    .unwrap()
                    .value_as_string(row),
            )
        };
    }
    Ok(match array.data_type() {
        DataType::Boolean => Value::bool(get!(array::BooleanArray)),
        DataType::Int8 => Value::i64(i64::from(get!(array::Int8Array))),
        DataType::Int16 => Value::i64(i64::from(get!(array::Int16Array))),
        DataType::Int32 => Value::i64(i64::from(get!(array::Int32Array))),
        DataType::Int64 => Value::i64(get!(array::Int64Array)),
        DataType::UInt8 => Value::i64(i64::from(get!(array::UInt8Array))),
        DataType::UInt16 => Value::i64(i64::from(get!(array::UInt16Array))),
        DataType::UInt32 => Value::i64(i64::from(get!(array::UInt32Array))),
        DataType::UInt64 => Value::u64(get!(array::UInt64Array)),
        DataType::Float16 => {
            let f = f32::from(get!(array::Float16Array));
            Value::f64(f64::from(f))
        }
        DataType::Float32 => Value::f64(f64::from(get!(array::Float32Array))),
        DataType::Float64 => Value::f64(get!(array::Float64Array)),
        DataType::Utf8 => Value::string(get!(array::StringArray)),
        DataType::LargeUtf8 => Value::string(get!(array::LargeStringArray)),
        DataType::Binary => Value::binary(get!(array::BinaryArray).to_vec()),
        DataType::LargeBinary => Value::binary(get!(array::LargeBinaryArray).to_vec()),
        DataType::FixedSizeBinary(_) => Value::binary(get!(array::FixedSizeBinaryArray).to_vec()),
        DataType::Decimal128(_, _) => decimal!(array::Decimal128Array),
        DataType::Decimal256(_, _) => decimal!(array::Decimal256Array),
        DataType::Timestamp(unit, _) => {
            let dt = match unit {
                TimeUnit::Second => array
                    .as_any()
                    .downcast_ref::<array::TimestampSecondArray>()
                    .unwrap()
                    .value_as_datetime(row),
                TimeUnit::Millisecond => array
                    .as_any()
                    .downcast_ref::<array::TimestampMillisecondArray>()
                    .unwrap()
                    .value_as_datetime(row),
                TimeUnit::Microsecond => array
                    .as_any()
                    .downcast_ref::<array::TimestampMicrosecondArray>()
                    .unwrap()
                    .value_as_datetime(row),
                TimeUnit::Nanosecond => array
                    .as_any()
                    .downcast_ref::<array::TimestampNanosecondArray>()
                    .unwrap()
                    .value_as_datetime(row),
            };
            match dt {
                Some(dt) => Value::string(dt.and_utc().to_rfc3339()),
                None => {
                    return Err(Error::InvalidArgument(
                        "timestamp is outside the representable range".to_string(),
                    ))
                }
            }
        }
        DataType::List(_) | DataType::LargeList(_) | DataType::FixedSizeList(_, _) => {
            return list(&elements_at(array, row))
        }
        DataType::Struct(fields) => {
            let columns = array.as_any().downcast_ref::<array::StructArray>().unwrap();
            let mut out = Vec::with_capacity(fields.len());
            for (i, field) in fields.iter().enumerate() {
                let value = value(columns.column(i), row)?;
                out.push((field.name().clone(), value));
            }
            return Ok(Value::r#struct(out));
        }
        other => Value::string(
            duckdb::arrow::util::display::array_value_to_string(&array, row).map_err(|e| {
                Error::InvalidArgument(format!("cannot render a {other} value as text: {e}"))
            })?,
        ),
    })
}

/// The element array of one list cell. Callers reach it only from a list
/// `DataType`, and arrow's three list layouts differ solely in their offset type.
fn elements_at(array: &ArrayRef, row: usize) -> ArrayRef {
    macro_rules! at {
        ($ty:ty) => {
            array.as_any().downcast_ref::<$ty>().unwrap().value(row)
        };
    }
    match array.data_type() {
        DataType::LargeList(_) => at!(array::LargeListArray),
        DataType::FixedSizeList(_, _) => at!(array::FixedSizeListArray),
        _ => at!(array::ListArray),
    }
}

fn list(elements: &ArrayRef) -> Result<Value, Error> {
    if elements.null_count() > 0 {
        return Err(Error::InvalidArgument(
            "list contains a null element, which TopK lists cannot hold".to_string(),
        ));
    }
    if elements.is_empty() {
        return Ok(Value::list(Vec::<f32>::new()));
    }
    let dtype = elements.data_type();
    Ok(match dtype {
        DataType::List(_) | DataType::LargeList(_) | DataType::FixedSizeList(_, _) => {
            return matrix(elements)
        }
        // A u64 past i64::MAX has no wider integer to read as.
        DataType::UInt64 => Value::list(cast(elements, &DataType::UInt64)?.as_primitive::<UInt64Type>().values().to_vec()),
        // `ty` picks the family; the elements are then read as its widest member.
        _ => match ty(dtype) {
            Type::Int => Value::list(
                cast(elements, &DataType::Int64)?
                    .as_primitive::<Int64Type>()
                    .values()
                    .to_vec(),
            ),
            Type::Float => Value::list(
                cast(elements, &DataType::Float64)?
                    .as_primitive::<Float64Type>()
                    .values()
                    .to_vec(),
            ),
            Type::Text
                if matches!(
                    dtype,
                    DataType::Utf8
                        | DataType::LargeUtf8
                        | DataType::Decimal128(_, _)
                        | DataType::Decimal256(_, _)
                ) =>
            {
                Value::list(
                    cast(elements, &DataType::Utf8)?
                        .as_string::<i32>()
                        .iter()
                        .map(|value| value.unwrap_or_default().to_string())
                        .collect::<Vec<String>>(),
                )
            }
            _ => {
                return Err(Error::InvalidArgument(format!(
                    "a list of {dtype} has no equivalent in TopK — lists hold numbers or strings. \
                     Flatten it in the source (a view can project the fields you need), or drop the field"
                )))
            }
        },
    })
}

/// The elements as `to`, which they are read as directly. Out of range is a
/// null in a safe cast, and a list holds no nulls by the time it gets here.
fn cast(elements: &ArrayRef, to: &DataType) -> Result<ArrayRef, Error> {
    let from = elements.data_type();
    let cast = duckdb::arrow::compute::cast(elements, to).map_err(|e| {
        Error::InvalidArgument(format!("cannot read a list of {from} as {to}: {e}"))
    })?;
    match cast.null_count() {
        0 => Ok(cast),
        _ => Err(Error::InvalidArgument(format!(
            "a list of {from} holds values that do not fit {to}"
        ))),
    }
}

fn matrix(rows: &ArrayRef) -> Result<Value, Error> {
    let rows: Vec<ArrayRef> = (0..rows.len()).map(|i| elements_at(rows, i)).collect();

    let mut values: Vec<f32> = Vec::new();
    let mut cols = 0usize;
    for (i, row) in rows.iter().enumerate() {
        let row: Vec<f32> = floats(&list(row)?)
            .map(|v| v.into_iter().map(|n| n as f32).collect())
            .ok_or_else(|| {
                Error::InvalidArgument("matrix rows must be numeric lists".to_string())
            })?;
        if i == 0 {
            cols = row.len();
        } else if row.len() != cols {
            return Err(Error::InvalidArgument(format!(
                "matrix rows differ in length ({cols} then {})",
                row.len()
            )));
        }
        values.extend(row);
    }
    if cols == 0 {
        return Err(Error::InvalidArgument(
            "matrix must have at least one column".to_string(),
        ));
    }
    Ok(Value::matrix(cols as u32, values))
}
