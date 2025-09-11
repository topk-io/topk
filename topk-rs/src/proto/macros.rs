#[macro_export]
macro_rules! schema {
    () => {
        std::collections::HashMap::default()
    };
    ($($field:expr => $spec:expr),* $(,)?) => {{
        std::collections::HashMap::from_iter([
            $(($field.to_string(), $spec)),*
        ])
    }};
}
