use duckdb::arrow::array::{self, Array, ArrayRef};
use duckdb::arrow::datatypes::{DataType, TimeUnit};
use topk_rs::proto::v1::data::Value;

use crate::import::error::Error;
use crate::import::spec::Type;
use crate::import::value::floats;

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
    macro_rules! at {
        ($ty:ty) => {{
            let a = elements.as_any().downcast_ref::<$ty>().unwrap();
            (0..a.len()).map(|i| a.value(i))
        }};
    }
    macro_rules! decimals {
        ($ty:ty) => {{
            let a = elements.as_any().downcast_ref::<$ty>().unwrap();
            Value::list(
                (0..a.len())
                    .map(|i| a.value_as_string(i))
                    .collect::<Vec<String>>(),
            )
        }};
    }
    Ok(match elements.data_type() {
        DataType::Int8 => Value::list(at!(array::Int8Array).map(i64::from).collect::<Vec<i64>>()),
        DataType::Int16 => Value::list(at!(array::Int16Array).map(i64::from).collect::<Vec<i64>>()),
        DataType::Int32 => Value::list(at!(array::Int32Array).map(i64::from).collect::<Vec<i64>>()),
        DataType::Int64 => Value::list(at!(array::Int64Array).collect::<Vec<i64>>()),
        DataType::UInt8 => Value::list(at!(array::UInt8Array).map(i64::from).collect::<Vec<i64>>()),
        DataType::UInt16 => {
            Value::list(at!(array::UInt16Array).map(i64::from).collect::<Vec<i64>>())
        }
        DataType::UInt32 => {
            Value::list(at!(array::UInt32Array).map(i64::from).collect::<Vec<i64>>())
        }
        DataType::UInt64 => Value::list(at!(array::UInt64Array).collect::<Vec<u64>>()),
        DataType::Float16 => {
            return Ok(Value::list(
                at!(array::Float16Array)
                    .map(|n| f64::from(f32::from(n)))
                    .collect::<Vec<_>>(),
            ));
        }
        DataType::Float32 => {
            return Ok(Value::list(
                at!(array::Float32Array)
                    .map(f64::from)
                    .collect::<Vec<_>>(),
            ));
        }
        DataType::Float64 => {
            return Ok(Value::list(
                at!(array::Float64Array).collect::<Vec<_>>(),
            ));
        }
        DataType::Utf8 => Value::list(
            at!(array::StringArray)
                .map(str::to_string)
                .collect::<Vec<String>>(),
        ),
        DataType::LargeUtf8 => Value::list(
            at!(array::LargeStringArray)
                .map(str::to_string)
                .collect::<Vec<String>>(),
        ),
        DataType::Decimal128(_, _) => decimals!(array::Decimal128Array),
        DataType::Decimal256(_, _) => decimals!(array::Decimal256Array),
        DataType::List(_) | DataType::LargeList(_) | DataType::FixedSizeList(_, _) => {
            return matrix(elements)
        }
        other => {
            return Err(Error::InvalidArgument(format!(
                "a list of {other} has no equivalent in TopK — lists hold numbers or strings. \
                 Flatten it in the source (a view can project the fields you need), or drop the field"
            )))
        }
    })
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
