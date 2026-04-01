use napi::bindgen_prelude::{FromNapiValue, ToNapiValue};
use std::ffi::CString;

/// Create an empty JS object via raw NAPI syscalls.
pub unsafe fn js_object(env: napi::sys::napi_env) -> napi::Result<napi::sys::napi_value> {
    let mut result = std::ptr::null_mut();
    napi::check_status!(napi::sys::napi_create_object(env, &mut result))?;
    Ok(result)
}

/// Set a named property on a JS object, converting the value via `ToNapiValue`.
pub unsafe fn js_set<V: ToNapiValue>(
    env: napi::sys::napi_env,
    obj: napi::sys::napi_value,
    key: &str,
    val: V,
) -> napi::Result<()> {
    let key_cstr = CString::new(key).map_err(|_| napi::Error::from_reason("invalid key"))?;
    let napi_val = V::to_napi_value(env, val)?;
    napi::check_status!(napi::sys::napi_set_named_property(
        env,
        obj,
        key_cstr.as_ptr(),
        napi_val
    ))?;
    Ok(())
}

// NapiBox

#[derive(Debug, Clone)]
pub struct NapiBox<T>(pub std::boxed::Box<T>);

impl<T> From<T> for NapiBox<T> {
    fn from(value: T) -> Self {
        Self(std::boxed::Box::new(value))
    }
}

impl<T> FromNapiValue for NapiBox<T>
where
    T: FromNapiValue,
{
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        napi_val: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        T::from_napi_value(env, napi_val).map(|v| Self(std::boxed::Box::new(v)))
    }
}

impl<T> ToNapiValue for NapiBox<T>
where
    T: ToNapiValue,
{
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        ToNapiValue::to_napi_value(env, *val.0)
    }
}

impl<T> AsRef<T> for NapiBox<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}
