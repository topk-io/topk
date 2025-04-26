fn main() {
    build_topk_v1();

    #[cfg(feature = "openapi")]
    build_openapi_spec();
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

    // let lib_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    builder
        .clone()
        // .file_descriptor_set_path(lib_dir.join("out/topk_v1_proto_descriptor_set.bin"))
        .compile_protos(
            &[
                "protos/topk/control/v1/collection_service.proto",
                "protos/topk/control/v1/collection.proto",
                "protos/topk/control/v1/schema.proto",
                "protos/topk/data/v1/document_service.proto",
                "protos/topk/data/v1/document.proto",
                "protos/topk/data/v1/query_service.proto",
                "protos/topk/data/v1/query.proto",
                "protos/topk/data/v1/value.proto",
                "protos/google/rpc/error_details.proto",
            ],
            &["protos/"],
        )
        .expect("failed to build [topk.v1] protos");
}

#[cfg(feature = "openapi")]
fn build_openapi_spec() {
    let out_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("out");

    // generate openapi spec
    let output = std::process::Command::new("protoc")
        .arg("--connect-openapi_opt=format=json")
        .arg(format!(
            "--connect-openapi_out={}",
            out_dir.to_string_lossy()
        ))
        .arg("--proto_path=protos")
        .arg("protos/topk/data/v1/query.proto")
        .arg("protos/topk/data/v1/document.proto")
        .arg("protos/topk/control/v1/collection.proto")
        .output()
        .expect("failed to generate [topk.v1] openapi spec");

    if !output.status.success() {
        panic!(
            "failed to generate [topk.v1] openapi spec: {}",
            String::from_utf8_lossy(&output.stderr),
        );
    }
}
