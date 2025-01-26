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
        topk_protos::v1::control::index_schema::IndexSchema::default()
    };
    ($($field:expr => $spec:expr),* $(,)?) => {{
        let schema = topk_protos::v1::control::index_schema::IndexSchema::try_from([
            $(($field, $spec)),*
        ]);

        schema.unwrap()
    }};
}
