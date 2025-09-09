use std::path::PathBuf;

use pb_rs::types::FileDescriptor;

fn main() {
    build_topk_v1_protos();
    build_topk_v1_quick_proto();
}

fn build_topk_v1_quick_proto() {
    let in_dir = PathBuf::from("/Users/marek/projects/fafolabs/ddb/sdk/protos");
    let out_dir = PathBuf::from("./src/qpb/topk/data/v1");

    println!("cargo:rerun-if-changed=build.rs");
    let proto_paths = [
        // "../protos/topk/control/v1/collection.proto",
        // "../protos/topk/control/v1/collection_service.proto",
        // "../protos/topk/control/v1/schema.proto",
        // "../protos/topk/data/v1/write_service.proto",
        "topk/data/v1/document.proto",
        // "../protos/topk/data/v1/query_service.proto",
        // "../protos/topk/data/v1/query.proto",
        "topk/data/v1/value.proto",
    ];

    for path in proto_paths {
        println!("cargo:rerun-if-changed={}", in_dir.join(path).display());
    }

    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir).unwrap();
    }
    std::fs::create_dir_all(&out_dir).unwrap();

    let cfg = pb_rs::ConfigBuilder::new(
        &proto_paths
            .iter()
            .map(|f| in_dir.join(f))
            .collect::<Vec<_>>(),
        None,
        Some(&out_dir),
        &[in_dir],
    )
    .unwrap()
    .single_module(true)
    .add_deprecated_fields(true)
    .gen_info(true)
    .build();

    FileDescriptor::run(&cfg).unwrap();
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
