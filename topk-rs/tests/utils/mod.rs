pub mod dataset;

mod test_context;
pub use test_context::project::ProjectTestContext;

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
