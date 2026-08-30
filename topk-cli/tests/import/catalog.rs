use std::collections::BTreeMap;

use crate::common::seed::{es, mongo, mysql, parquet, pg, sqlite, Seed};
use crate::common::*;
use rstest::rstest;

#[rstest]
#[case::sqlite(Box::new(sqlite::Db::new().unwrap()))]
#[case::postgres(Box::new(pg::Pg::new().unwrap()))]
#[case::mysql(Box::new(mysql::MySql::new().unwrap()))]
#[case::mongo(Box::new(mongo::Mongo::client()))]
#[case::elasticsearch(Box::new(es::Es::client()))]
#[case::parquet(Box::new(parquet::File::new().unwrap()))]
#[tokio::test]
async fn discovered_types(#[case] backend: Box<dyn Seed>) {
    let name = unique_name("books");
    let object = backend.seed(&name, books()).await.unwrap();
    let locator = backend.url().unwrap_or_else(|| object.from.clone());

    let spec = discover_spec(&locator, Some(&object.from)).await;
    let target = spec
        .collections
        .get(name.as_str())
        .unwrap_or_else(|| panic!("no target {name}"));

    assert_eq!(target.id.as_deref(), Some("_id"));
    assert_eq!(
        target.fields.len(),
        5,
        "unexpected fields: {:?}",
        target.fields.keys().collect::<Vec<_>>()
    );
    // `in_print` is pinned separately: sqlite's affinity typing reports int.
    let types: BTreeMap<&str, String> = target
        .fields
        .iter()
        .filter(|(name, _)| name.as_str() != "in_print")
        .map(|(name, field)| (name.as_str(), field.ty.to_string()))
        .collect();
    assert_eq!(
        types,
        BTreeMap::from([
            ("author", "text".to_string()),
            ("published_year", "int".to_string()),
            ("rating", "float".to_string()),
            ("title", "text".to_string()),
        ])
    );
}

// sqlite type names are affinity hints: a BOOLEAN column surfaces as int.
#[tokio::test]
async fn sqlite_bool_is_int() {
    let db = sqlite::Db::new().unwrap();
    let name = unique_name("books");
    let object = db.seed(&name, books()).await.unwrap();

    let spec = discover_spec(&db.url().expect("database url"), Some(&object.from)).await;
    assert_eq!(
        spec.collections[name.as_str()].fields["in_print"]
            .ty
            .to_string(),
        "int"
    );
}
