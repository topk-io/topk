use serde::Serialize;

pub fn inline<T: Serialize>(value: &T) -> String {
    toml::Value::try_from(value)
        .map(|value| value.to_string())
        .unwrap_or_default()
}
