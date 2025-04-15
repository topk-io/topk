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
