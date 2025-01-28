#[macro_export]
macro_rules! doc {
    ($($field:expr => $value:expr),* $(,)?) => {
        topk_protos::v1::data::Document::from([
            $(($field, $value.into())),*
        ])
    };
}

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
