use std::collections::BTreeMap;

use serde::Serialize;

use crate::import::preview::elide;
use crate::import::spec::{Spec, Type};

/// What prints here is what `--spec` would re-run. `fresh` is None before a
/// cluster has been consulted (--dry-run); `after` holds the resume cursors.
pub fn render(spec: &Spec, fresh: Option<&[&str]>, after: &BTreeMap<String, String>) -> String {
    let mut out = String::new();
    let mut indexed: Vec<String> = Vec::new();
    // An unindexed float_list imports fine and silently is not searchable.
    let lists = spec
        .collections
        .values()
        .flat_map(|target| target.fields.values())
        .any(|field| matches!(field.ty, Type::FloatList) && field.index.is_none());
    for (name, target) in spec.collections.iter() {
        if let Some(fresh) = fresh {
            let state = match fresh.contains(&name.as_str()) {
                true => "will create",
                false => "exists",
            };
            out.push_str(&format!("# {state}\n"));
        }
        if let Some(mark) = after.get(name) {
            out.push_str(&format!(
                "# resuming after {}\n",
                elide(&serde_json::Value::String(mark.clone()))
            ));
        }
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
            if let Some(index) = &spec.index {
                indexed.push(format!("{field} ({})", inline(index).trim_matches('"')));
            }
        }
        out.push('\n');
    }
    match indexed.is_empty() {
        true => out.push_str(
            "# no indexes declared — the data will import but will not be searchable. Add one:\n\
             #   text    index = \"keyword\" | \"exact\" | \"semantic\" | \"ngram\"\n\
             #   vector  index = { vector = { metric = \"cosine\" } }\n\
             #   matrix  index = { multi_vector = {} }\n",
        ),
        false => out.push_str(&format!("# indexed: {}\n", indexed.join(", "))),
    }
    if lists {
        out.push_str(
            "# a float_list is not searchable as-is; for vector search use: \
             { type = \"f32_vector\", dim = <N>, index = { vector = { metric = \"cosine\" } } }\n",
        );
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
