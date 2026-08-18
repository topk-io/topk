use topk_rs::proto::v1::data::{value::Value as Inner, Document};

pub struct Dialect {
    pub text: &'static str,
    pub int: &'static str,
    pub float: &'static str,
}

pub const POSTGRES: Dialect = Dialect {
    text: "text",
    int: "bigint",
    float: "double precision",
};

pub const MYSQL: Dialect = Dialect {
    text: "varchar(255)",
    int: "bigint",
    float: "double",
};

pub const SQLITE: Dialect = Dialect {
    text: "text",
    int: "integer",
    float: "real",
};

// `_id` leads so the seeded table reads like the documents it came from; the
// rest are sorted to keep the DDL stable across runs.
fn columns(docs: &[Document]) -> Vec<&String> {
    let first = &docs.first().expect("at least one document").fields;
    let mut names: Vec<&String> = first.keys().collect();
    names.sort_by_key(|name| (name.as_str() != "_id", name.as_str()));
    names
}

fn column_type(value: Option<&topk_rs::proto::v1::data::Value>, dialect: &Dialect) -> &'static str {
    match value.and_then(|v| v.value.as_ref()) {
        Some(Inner::Bool(_)) => "boolean",
        Some(Inner::I32(_) | Inner::I64(_) | Inner::U32(_) | Inner::U64(_)) => dialect.int,
        Some(Inner::F32(_) | Inner::F64(_)) => dialect.float,
        _ => dialect.text,
    }
}

fn literal(value: Option<&topk_rs::proto::v1::data::Value>) -> String {
    match value.and_then(|v| v.value.as_ref()) {
        Some(Inner::Bool(b)) => b.to_string(),
        Some(Inner::I32(n)) => n.to_string(),
        Some(Inner::I64(n)) => n.to_string(),
        Some(Inner::U32(n)) => n.to_string(),
        Some(Inner::U64(n)) => n.to_string(),
        Some(Inner::F32(f)) => f.to_string(),
        Some(Inner::F64(f)) => f.to_string(),
        Some(Inner::String(s)) => format!("'{}'", s.replace('\'', "''")),
        _ => "NULL".to_string(),
    }
}

pub fn ddl(table: &str, docs: &[Document], dialect: &Dialect) -> String {
    let first = &docs[0].fields;
    let columns = columns(docs)
        .into_iter()
        .map(|name| format!("{name} {}", column_type(first.get(name), dialect)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("CREATE TABLE {table} ({columns})")
}

pub fn insert(table: &str, docs: &[Document]) -> String {
    let names = columns(docs);
    let rows = docs
        .iter()
        .map(|doc| {
            let values = names
                .iter()
                .map(|name| literal(doc.fields.get(*name)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({values})")
        })
        .collect::<Vec<_>>()
        .join(", ");
    let columns = names
        .iter()
        .map(|name| name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!("INSERT INTO {table} ({columns}) VALUES {rows}")
}
