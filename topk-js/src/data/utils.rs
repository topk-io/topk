use napi::bindgen_prelude::*;

pub unsafe fn is_napi_integer(env: napi::sys::napi_env, napi_val: napi::sys::napi_value) -> bool {
    // Check if the number is an integer by comparing it with its integer part
    let num = f64::from_napi_value(env, napi_val).unwrap();

    if num == (num as i64) as f64 {
        // It's an integer (no fractional part)
        true
    } else {
        // It has a fractional part, so it's a float
        false
    }
}

pub fn get_napi_value_type(value_type: i32) -> String {
    match value_type {
        napi::sys::ValueType::napi_undefined => "undefined".to_string(),
        napi::sys::ValueType::napi_null => "null".to_string(),
        napi::sys::ValueType::napi_boolean => "boolean".to_string(),
        napi::sys::ValueType::napi_number => "number".to_string(),
        napi::sys::ValueType::napi_symbol => "symbol".to_string(),
        napi::sys::ValueType::napi_string => "string".to_string(),
        napi::sys::ValueType::napi_object => "object".to_string(),
        napi::sys::ValueType::napi_function => "function".to_string(),
        napi::sys::ValueType::napi_external => "external".to_string(),
        _ => "unknown".to_string(),
    }
}
