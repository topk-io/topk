mod utils;

mod client;
mod data;
mod error;
mod expr;
mod query;
mod schema;

#[macro_export]
macro_rules! try_cast_ref {
    ($env:expr, $obj:expr, $type:ty) => {{
        let obj = Unknown::from_napi_value($env, $obj)?;

        let env = napi::Env::from_raw($env);
        let is_instance = <$type>::instance_of(env, &obj)?;

        if is_instance {
            <$type>::from_napi_ref($env, $obj)
        } else {
            Err(napi::Error::from_reason("Invalid type"))
        }
    }};
}
