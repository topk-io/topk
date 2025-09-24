fn main() {
    // Rerun if build.rs changes
    println!("cargo::rerun-if-changed=build.rs");
    build_topk_v1_protos();
}

fn build_topk_v1_protos() {
    let proto_paths = [
        "../protos/topk/control/v1/collection_service.proto",
        "../protos/topk/control/v1/collection.proto",
        "../protos/topk/control/v1/schema.proto",
        "../protos/topk/data/v1/write_service.proto",
        "../protos/topk/data/v1/document.proto",
        "../protos/topk/data/v1/query_service.proto",
        "../protos/topk/data/v1/query.proto",
        "../protos/topk/data/v1/value.proto",
    ];

    // Rerun if any proto file changes
    for path in proto_paths {
        println!("cargo::rerun-if-changed={}", path);
    }

    let mut builder = tonic_prost_build::configure();

    // #[derive(serde::Serialize, serde::Deserialize)]
    for message in [
        // field spec
        "topk.control.v1.FieldSpec",
        // field type
        "topk.control.v1.FieldType",
        "topk.control.v1.FieldType.data_type",
        "topk.control.v1.FieldTypeF32Vector",
        "topk.control.v1.FieldTypeU8Vector",
        "topk.control.v1.FieldTypeI8Vector",
        "topk.control.v1.FieldTypeBinaryVector",
        "topk.control.v1.FieldTypeF32SparseVector",
        "topk.control.v1.FieldTypeU8SparseVector",
        "topk.control.v1.FieldTypeBoolean",
        "topk.control.v1.FieldTypeInteger",
        "topk.control.v1.FieldTypeFloat",
        "topk.control.v1.FieldTypeText",
        "topk.control.v1.FieldTypeBytes",
        "topk.control.v1.FieldTypeList",
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
        .codec_path("crate::proto::codec::ProstCodec")
        .bytes(".topk.data.v1.Value")
        .bytes(".topk.data.v1.DocumentData")
        .compile_protos(&proto_paths, &["../protos/"])
        .expect("failed to build [topk.v1] protos");
}
