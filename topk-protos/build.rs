use std::{env, path::PathBuf};

fn main() {
    build_topk_v1();

    #[cfg(target_os = "macos")]
    build_openapi_spec();
}

fn build_topk_v1() {
    let lib_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let mut builder = tonic_build::configure();

    // #[derive(Eq, Hash)] for messages
    for message in vec![
        // field spec
        "topk.control.v1.FieldSpec",
        // field type
        "topk.control.v1.FieldType",
        "topk.control.v1.FieldType.data_type",
        "topk.control.v1.FieldTypeFloatVector",
        "topk.control.v1.FieldTypeByteVector",
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
        "topk.control.v1.FieldTypeFloatVector",
        "topk.control.v1.FieldTypeByteVector",
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
    ] {
        builder =
            builder.type_attribute(message, "#[derive(serde::Serialize, serde::Deserialize)]");
    }

    builder
        .clone()
        .file_descriptor_set_path(lib_dir.join("out/topk_v1_proto_descriptor_set.bin"))
        .compile_protos(
            &[
                "protos/topk/control/v1/index.proto",
                "protos/topk/data/v1/value.proto",
                "protos/topk/data/v1/document.proto",
                "protos/topk/data/v1/query.proto",
                "protos/google/rpc/error_details.proto",
            ],
            &["protos/"],
        )
        .expect("failed to build [topk.v1] protos");
}

#[cfg(target_os = "macos")]
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
        .arg("protos/topk/control/v1/index.proto")
        .output()
        .expect("failed to generate [topk.v1] openapi spec");

    if !output.status.success() {
        panic!(
            "failed to generate [topk.v1] openapi spec: {}",
            String::from_utf8_lossy(&output.stderr),
        );
    }
}
