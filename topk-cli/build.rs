fn main() {
    tonic_prost_build::configure()
        .type_attribute("topk.management.v1.Project", "#[derive(serde::Serialize)]")
        .extern_path(".topk.control.v1", "::topk_rs::proto::v1::control")
        .compile_protos(
            &[
                "../protos/topk/management/v1/project.proto",
                "../protos/topk/management/v1/collection.proto",
                "../protos/topk/management/v1/region.proto",
            ],
            &["../protos"],
        )
        .expect("failed to build management API protos");
}
