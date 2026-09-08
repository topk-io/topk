use serde::Serialize;

use crate::import::spec::Spec;

/// What prints here is what `--spec` would re-run.
pub fn render(spec: &Spec) -> String {
    let mut out = String::new();
    for (name, target) in spec.collections.iter() {
        out.push_str(&format!("[{}]\n", key(name)));
        for (field, value) in [
            ("from", Some(target.from.as_str())),
            ("id", target.id.as_deref()),
            ("filter", target.filter.as_deref()),
            ("partition", target.partition.as_deref()),
        ] {
            if let Some(value) = value {
                out.push_str(&format!("{field} = {}\n", string(value)));
            }
        }
        if let Some(limit) = target.limit {
            out.push_str(&format!("limit = {limit}\n"));
        }
        out.push_str(&format!("\n[{}.fields]\n", key(name)));
        for (field, spec) in target.fields.iter() {
            out.push_str(&format!("{} = {}\n", key(field), inline(spec)));
        }
        out.push('\n');
    }
    out
}

pub fn inline<T: Serialize>(value: &T) -> String {
    toml::Value::try_from(value)
        .map(|value| value.to_string())
        .unwrap_or_default()
}

fn key(name: &str) -> String {
    match name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
    {
        true => name.to_string(),
        false => string(name),
    }
}

fn string(value: &str) -> String {
    toml::Value::String(value.to_string()).to_string()
}
