use napi::bindgen_prelude::{FromNapiValue, ToNapiValue};

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
