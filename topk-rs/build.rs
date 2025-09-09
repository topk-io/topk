use std::process::Command;

fn main() {
    build_topk_v1_protos();
    build_topk_v1_flatbuffers();
}

fn build_topk_v1_protos() {
    let mut builder = tonic_prost_build::configure();

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
        .compile_protos(
            &[
                "../protos/topk/control/v1/collection_service.proto",
                "../protos/topk/control/v1/collection.proto",
                "../protos/topk/control/v1/schema.proto",
                "../protos/topk/data/v1/write_service.proto",
                "../protos/topk/data/v1/document.proto",
                "../protos/topk/data/v1/query_service.proto",
                "../protos/topk/data/v1/query.proto",
                "../protos/topk/data/v1/value.proto",
            ],
            &["../protos/"],
        )
        .expect("failed to build [topk.v1] protos");
}

fn build_topk_v1_flatbuffers() {
    // Compile all .fbs files with:
    // flatc -o ./bob-flatbuffers/src --filename-suffix '' --rust ./bob-flatbuffers/flatbuffers/*.fbs

    // Check flatbuffers compiler is installed
    let code = Command::new("which")
        .arg("flatc")
        .status()
        .expect("flatc not installed");
    if !code.success() {
        panic!("flatc not installed");
    }

    // List of flatbuffers specs to compile
    let fbs_files = ["../flatbuffers/v1/document.fbs"];

    // Compile specs that have changed
    for fbs_path in fbs_files {
        println!("cargo:rerun-if-changed={fbs_path}");
        let res = Command::new("flatc")
            .arg("-o")
            .arg("./src/flatbuffers/v1")
            .arg("--filename-suffix")
            .arg("")
            .arg("--rust")
            .arg(fbs_path)
            .output()
            .expect("failed to comile flatbuffers spec");

        if !res.status.success() {
            panic!(
                "failed to compile flatbuffers spec. {}",
                std::str::from_utf8(&res.stderr).unwrap()
            );
        }
    }

    Command::new("cargo")
        .arg("fmt")
        .output()
        .expect("failed to format code");
}
