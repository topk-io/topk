use napi::{bindgen_prelude::*, Error, Status};
use std::{ffi::CString, iter::zip, ptr};

#[derive(Debug, Clone, PartialEq)]
pub struct SparseVectorU8 {
    pub(crate) indices: Vec<u32>,
    pub(crate) values: Vec<u8>,
}

impl ToNapiValue for SparseVectorU8 {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        let mut object = ptr::null_mut();
        check_status!(
            napi::sys::napi_create_object(env, &mut object),
            "Failed to create JavaScript object"
        )?;

        for (k, v) in zip(val.indices, val.values) {
            let key = CString::new(k.to_string())?;
            let value = u8::to_napi_value(env, v)?;

            check_status!(
                napi::sys::napi_set_named_property(env, object, key.as_ptr(), value),
                "Failed to set property"
            )?;
        }

        Ok(object)
    }
}

impl FromNapiValue for SparseVectorU8 {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        let mut data_type: i32 = 0;
        napi::sys::napi_typeof(env, value, &mut data_type);

        match data_type {
            napi::sys::ValueType::napi_object => {
                let object = Object::from_napi_value(env, value)?;

                let mut indices = Vec::new();
                let mut values = Vec::new();

                for key in Object::keys(&object)? {
                    let key = key.parse::<u32>().map_err(|_| {
                        Error::new(Status::InvalidArg, "Invalid key, must be u32".to_string())
                    })?;
                    indices.push(key);

                    let value = object
                        .get_named_property::<u8>(&key.to_string())
                        .map_err(|_| Error::new(Status::InvalidArg, "Invalid value, must be u8"))?;

                    values.push(value);
                }

                Ok(SparseVectorU8 { indices, values })
            }
            _ => Err(Error::new(
                Status::InvalidArg,
                "Invalid vector. Expected an object with keys and values.".to_string(),
            )),
        }
    }
}
