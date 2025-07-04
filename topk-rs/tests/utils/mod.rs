use topk_rs::proto::v1::data::Document;

pub mod dataset;

mod test_context;
pub use test_context::project::ProjectTestContext;

#[allow(dead_code)]
pub fn is_sorted(result: &[Document], field: &str) -> bool {
    result
        .iter()
        .map(|d| {
            d.fields
                .get(field)
                .and_then(|v| v.as_f32())
                .expect("missing sorting field")
        })
        .is_sorted()
}

#[macro_export]
macro_rules! assert_doc_ids {
    ($docs:expr, $expected:expr) => {{
        let ids = $docs
            .into_iter()
            .map(|d| d.id().unwrap().to_string())
            .collect::<std::collections::HashSet<_>>();

        let expected = $expected
            .into_iter()
            .map(|s| s.into())
            .collect::<std::collections::HashSet<_>>();

        assert!(
            ids == expected,
            "actual: {:?}, expected: {:?}",
            ids,
            expected
        );
    }};
}

#[macro_export]
macro_rules! assert_doc_ids_ordered {
    ($docs:expr, $expected:expr) => {{
        let ids = $docs
            .into_iter()
            .map(|d| d.id().unwrap().to_string())
            .collect::<Vec<_>>();

        let expected = Vec::from($expected);

        assert!(
            ids == expected,
            "actual: {:?}, expected: {:?}",
            ids,
            expected
        );
    }};
}

#[macro_export]
macro_rules! assert_fields {
    ($docs:expr, $expected:expr) => {{
        let fields = $docs
            .into_iter()
            .flat_map(|d| d.fields.keys().map(|k| k.as_str()))
            .collect::<std::collections::HashSet<_>>();

        let expected = $expected
            .into_iter()
            .map(|s| s.into())
            .collect::<std::collections::HashSet<_>>();

        assert!(
            fields == expected,
            "actual: {:?}, expected: {:?}",
            fields,
            expected
        );
    }};
}
