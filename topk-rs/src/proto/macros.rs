#[macro_export]
macro_rules! doc {
    ($($field:expr => $value:expr),* $(,)?) => {
        topk_rs::proto::v1::data::Document::from([
            $(($field, $value.into())),*
        ])
    };
}

#[macro_export]
macro_rules! r#struct {
    ($($field:expr => $value:expr),* $(,)?) => {
        topk_rs::proto::v1::data::Value::r#struct(
            std::collections::HashMap::from_iter(
                [$(($field.to_string(), $value.into())),*]
            )
        )
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
