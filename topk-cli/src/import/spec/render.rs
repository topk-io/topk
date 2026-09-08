use toml_edit::{DocumentMut, Item, Value};

use crate::import::spec::Spec;

/// What prints here is what `--spec` would re-run. Serde writes every field as
/// its own table; a spec is read and edited by hand, so they fold to one line.
pub fn render(spec: &Spec) -> String {
    let mut doc: DocumentMut = toml::to_string(spec)
        .expect("a spec serializes")
        .parse()
        .expect("serde writes valid toml");
    for (_, target) in doc.iter_mut() {
        let Some(fields) = target
            .as_table_mut()
            .and_then(|target| target.get_mut("fields"))
            .and_then(Item::as_table_mut)
        else {
            continue;
        };
        for (mut key, field) in fields.iter_mut() {
            if let Some(table) = field.as_table() {
                key.fmt();
                let mut table = table.clone().into_inline_table();
                // A document's key is always text; saying so reads as a choice.
                if key.get() == crate::import::ID {
                    table.remove("type");
                }
                *field = Item::Value(Value::InlineTable(table));
            }
        }
    }
    doc.to_string()
}

pub fn inline<T: serde::Serialize>(value: &T) -> String {
    toml::Value::try_from(value)
        .map(|value| value.to_string())
        .unwrap_or_default()
}
