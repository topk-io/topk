fn main() {
    build_topk_v1();
}

fn build_topk_v1() {
    let mut builder = tonic_build::configure();

    // #[derive(Eq, Hash)] for messages
    for message in vec![
        // field spec
        "topk.control.v1.FieldSpec",
        // field type
        "topk.control.v1.FieldType",
        "topk.control.v1.FieldType.data_type",
        "topk.control.v1.FieldTypeF32Vector",
        "topk.control.v1.FieldTypeU8Vector",
        "topk.control.v1.FieldTypeBinaryVector",
        "topk.control.v1.FieldTypeBoolean",
        "topk.control.v1.FieldTypeInteger",
        "topk.control.v1.FieldTypeFloat",
        "topk.control.v1.FieldTypeText",
        "topk.control.v1.FieldTypeBytes",
        // index
        "topk.control.v1.FieldIndex",
        "topk.control.v1.FieldIndex.index",
        "topk.control.v1.KeywordIndex",
        "topk.control.v1.VectorIndex",
        "topk.control.v1.SemanticIndex",
    ] {
        builder = builder.type_attribute(message, "#[derive(Eq, Hash)]");
    }

    // #[derive(serde::Serialize, serde::Deserialize)]
    for message in vec![
        // field spec
        "topk.control.v1.FieldSpec",
        // field type
        "topk.control.v1.FieldType",
        "topk.control.v1.FieldType.data_type",
        "topk.control.v1.FieldTypeF32Vector",
        "topk.control.v1.FieldTypeU8Vector",
        "topk.control.v1.FieldTypeBinaryVector",
        "topk.control.v1.FieldTypeBoolean",
        "topk.control.v1.FieldTypeInteger",
        "topk.control.v1.FieldTypeFloat",
        "topk.control.v1.FieldTypeText",
        "topk.control.v1.FieldTypeBytes",
        // indexes
        "topk.control.v1.FieldIndex",
        "topk.control.v1.FieldIndex.index",
        "topk.control.v1.KeywordIndex",
        "topk.control.v1.VectorIndex",
        "topk.control.v1.SemanticIndex",
    ] {
        builder =
            builder.type_attribute(message, "#[derive(serde::Serialize, serde::Deserialize)]");
    }

    builder
        .clone()
        .compile_protos(
            &[
                "protos/topk/control/v1/collection_service.proto",
                "protos/topk/control/v1/collection.proto",
                "protos/topk/control/v1/schema.proto",
                "protos/topk/data/v1/write_service.proto",
                "protos/topk/data/v1/document.proto",
                "protos/topk/data/v1/query_service.proto",
                "protos/topk/data/v1/query.proto",
                "protos/topk/data/v1/value.proto",
            ],
            &["protos/"],
        )
        .expect("failed to build [topk.v1] protos");
}
