mod document;
mod list;
mod value;

pub use document::*;
pub use list::*;
pub use value::*;

#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    #[error("Missing document _id field")]
    MissingId,

    #[error("Invalid document _id field: {0:?}")]
    InvalidId(Value),
}

#[macro_export]
macro_rules! doc {
    ($($field:expr => $value:expr),* $(,)?) => {
        $crate::Document::from([
            $(($field, $value.into())),*
        ])
    };
}

#[macro_export]
macro_rules! r#struct {
    ($($field:expr => $value:expr),* $(,)?) => {
        $crate::Value::Struct(
            std::collections::HashMap::from_iter([
                $(($field.to_string(), $value.into())),*
            ])
        )
    };
}

pub trait ScalarType
where
    Self: Sized,
{
    fn to_value(value: Self) -> Value;

    fn to_list(values: Vec<Self>) -> ListValue;

    fn as_slice(list: &ListValue) -> Option<&[Self]>;
}

macro_rules! impl_scalar_type {
    ($type:ty, $enum_variant:ident) => {
        impl ScalarType for $type {
            #[inline(always)]
            fn to_value(value: Self) -> Value {
                Value::$enum_variant(value)
            }

            #[inline(always)]
            fn to_list(values: Vec<Self>) -> ListValue {
                ListValue::$enum_variant(values)
            }

            #[inline(always)]
            fn as_slice(list: &ListValue) -> Option<&[Self]> {
                match list {
                    ListValue::$enum_variant(value) => Some(value),
                    _ => None,
                }
            }
        }
    };
}

// Unsigned integer
impl_scalar_type!(u8, U8);
impl_scalar_type!(u16, U16);
impl_scalar_type!(u32, U32);
impl_scalar_type!(u64, U64);

// Signed integer
impl_scalar_type!(i8, I8);
impl_scalar_type!(i16, I16);
impl_scalar_type!(i32, I32);
impl_scalar_type!(i64, I64);

// Floating point
impl_scalar_type!(f32, F32);
impl_scalar_type!(f64, F64);

// String
impl_scalar_type!(String, String);
