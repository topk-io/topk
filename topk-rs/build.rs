fn main() {
    // Rerun if build.rs changes
    println!("cargo::rerun-if-changed=build.rs");
    // Rerun if EARTHLY_GIT_HASH changes
    println!("cargo::rerun-if-env-changed=EARTHLY_GIT_HASH");

    build_topk_v1_protos();
}

fn build_topk_v1_protos() {
    let proto_paths = [
        "../protos/topk/control/v1/collection_service.proto",
        "../protos/topk/control/v1/collection.proto",
        "../protos/topk/control/v1/schema.proto",
        "../protos/topk/control/v1/dataset_service.proto",
        "../protos/topk/control/v1/dataset.proto",
        "../protos/topk/data/v1/write_service.proto",
        "../protos/topk/data/v1/document.proto",
        "../protos/topk/data/v1/query_service.proto",
        "../protos/topk/data/v1/query.proto",
        "../protos/topk/data/v1/value.proto",
        "../protos/topk/data/v1/expr/function.proto",
        "../protos/topk/data/v1/expr/logical.proto",
        "../protos/topk/data/v1/expr/text.proto",
        "../protos/topk/ctx/v1/dataset_read_service.proto",
        "../protos/topk/ctx/v1/dataset_write_service.proto",
        "../protos/topk/ctx/v1/context_service.proto",
    ];

    // Rerun if any proto file changes
    for path in proto_paths {
        println!("cargo::rerun-if-changed={}", path);
    }

    let mut builder = tonic_prost_build::configure();

    // #[derive(serde::Serialize, serde::Deserialize)]
    for message in [
        // data Value
        "topk.data.v1.Value",
        "topk.data.v1.Value.value",
        "topk.data.v1.List",
        "topk.data.v1.List.values",
        "topk.data.v1.List.U8",
        "topk.data.v1.List.I8",
        "topk.data.v1.List.U32",
        "topk.data.v1.List.U64",
        "topk.data.v1.List.I32",
        "topk.data.v1.List.I64",
        "topk.data.v1.List.F8",
        "topk.data.v1.List.F16",
        "topk.data.v1.List.F32",
        "topk.data.v1.List.F64",
        "topk.data.v1.List.String",
        "topk.data.v1.Struct",
        "topk.data.v1.Vector",
        "topk.data.v1.Vector.vector",
        "topk.data.v1.Vector.Float",
        "topk.data.v1.Vector.Byte",
        "topk.data.v1.SparseVector",
        "topk.data.v1.SparseVector.values",
        "topk.data.v1.SparseVector.F32Values",
        "topk.data.v1.SparseVector.U8Values",
        "topk.data.v1.Matrix",
        "topk.data.v1.Matrix.values",
        "topk.data.v1.Matrix.F32",
        "topk.data.v1.Matrix.F16",
        "topk.data.v1.Matrix.F8",
        "topk.data.v1.Matrix.U8",
        "topk.data.v1.Matrix.I8",
        "topk.data.v1.Null",
        // field spec
        "topk.control.v1.FieldSpec",
        // field type
        "topk.control.v1.FieldType",
        "topk.control.v1.FieldType.data_type",
        "topk.control.v1.FieldTypeF32Vector",
        "topk.control.v1.FieldTypeF16Vector",
        "topk.control.v1.FieldTypeF8Vector",
        "topk.control.v1.FieldTypeU8Vector",
        "topk.control.v1.FieldTypeI8Vector",
        "topk.control.v1.FieldTypeBinaryVector",
        "topk.control.v1.FieldTypeF32SparseVector",
        "topk.control.v1.FieldTypeU8SparseVector",
        "topk.control.v1.FieldTypeMatrix",
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
        "topk.control.v1.MultiVectorIndex",
        // ctx
        "topk.ctx.v1.AskResult",
        "topk.ctx.v1.AskResult.message",
        "topk.ctx.v1.AskResult.Search",
        "topk.ctx.v1.AskResult.Reason",
        "topk.ctx.v1.AskResult.Answer",
        "topk.ctx.v1.Fact",
        "topk.ctx.v1.SearchResult",
        "topk.ctx.v1.Content",
        "topk.ctx.v1.Content.data",
        "topk.ctx.v1.Chunk",
        "topk.ctx.v1.Page",
        "topk.ctx.v1.Image",
    ] {
        builder =
            builder.type_attribute(message, "#[derive(serde::Serialize, serde::Deserialize)]");
    }

    builder
        .codec_path("crate::proto::codec::ProstCodec")
        .bytes(".topk.data.v1.Value")
        .bytes(".topk.data.v1.DocumentData")
        .bytes(".topk.ctx.v1.UpsertMessage.BodyChunk.data")
        .bytes(".topk.ctx.v1.Image.data")
        .compile_protos(&proto_paths, &["../protos/"])
        .expect("failed to build [topk.v1] protos");
}
