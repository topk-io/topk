use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;

use crate::Error;

pub struct Args<'a>(pub &'a [String]);

impl<'a> Args<'a> {
    pub fn get<T: FromStr>(&self, i: usize) -> Result<T, Error>
    where
        T::Err: Display,
    {
        let val = self.0.get(i).ok_or_else(|| {
            Error::Invalid(format!(
                "function requires {} argument(s), got {}",
                i + 1,
                self.0.len()
            ))
        })?;
        val.parse::<T>()
            .map_err(|e| Error::Invalid(format!("argument {} is invalid: {e}", i)))
    }

    pub fn done(&self, expected: usize) -> Result<(), Error> {
        if self.0.len() != expected {
            return Err(Error::Invalid(format!(
                "function expects {} argument(s), got {}",
                expected,
                self.0.len()
            )));
        }
        Ok(())
    }
}

pub struct Kwargs<'a>(pub &'a HashMap<String, String>);

impl<'a> Kwargs<'a> {
    pub fn required<T: FromStr>(&self, key: &str) -> Result<T, Error>
    where
        T::Err: Display,
    {
        let val = self
            .0
            .get(key)
            .ok_or_else(|| Error::Invalid(format!("missing required option `{key}`")))?;
        val.parse::<T>()
            .map_err(|e| Error::Invalid(format!("option `{key}` is invalid: {e}")))
    }

    pub fn optional<T: FromStr>(&self, key: &str) -> Result<Option<T>, Error>
    where
        T::Err: Display,
    {
        self.0
            .get(key)
            .map(|s| {
                s.parse::<T>()
                    .map_err(|e| Error::Invalid(format!("option `{key}` is invalid: {e}")))
            })
            .transpose()
    }

    pub fn done(self, known: &[&str]) -> Result<(), Error> {
        for key in self.0.keys() {
            if !known.contains(&key.as_str()) {
                return Err(Error::Invalid(format!("unknown option `{key}`")));
            }
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! parse_args {
    ($args:expr; $($t:ty),+) => {{
        let a = $crate::util::Args($args);
        let mut i = 0usize;
        let result = ( $( { let v = a.get::<$t>(i)?; i += 1; v } ),+ ,);
        a.done(i)?;
        Ok::<_, Error>(result)
    }};
}

#[macro_export]
macro_rules! parse_kwargs {
    // Required only: parse_kwargs!(&opts; key: Type, ...)
    ($opts:expr; $($key:ident: $t:ty),+ $(,)?) => {{
        let k = $crate::util::Kwargs($opts);
        let result = ( $( k.required::<$t>(stringify!($key))? ),+ ,);
        k.done(&[$(stringify!($key)),+])?;
        Ok::<_, $crate::Error>(result)
    }};
    // Required + optional: parse_kwargs!(&opts; req: T; opt: T, ...)
    ($opts:expr; $($rk:ident: $rt:ty),+; $($ok:ident: $ot:ty),+ $(,)?) => {{
        let k = $crate::util::Kwargs($opts);
        let result = (
            $( k.required::<$rt>(stringify!($rk))? ),+ ,
            $( k.optional::<$ot>(stringify!($ok))? ),+ ,
        );
        k.done(&[$(stringify!($rk)),+, $(stringify!($ok)),+])?;
        Ok::<_, $crate::Error>(result)
    }};
}
