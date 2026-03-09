use std::path::PathBuf;
use std::time::Duration;

use topk_rs::client::WaitConfig;
use topk_rs::proto::v1::ctx::file::InputFile;

pub mod books;
pub mod multi_vec;
pub mod semantic;

#[allow(dead_code)]
pub fn quick_wait() -> Option<WaitConfig> {
    Some(WaitConfig {
        frequency: Duration::from_secs(1),
        timeout: Duration::from_secs(15),
    })
}

#[allow(dead_code)]
pub fn test_pdf() -> InputFile {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("utils")
        .join("dataset")
        .join("pdfko.pdf");

    InputFile::from_path(&path).expect("could not create InputFile from path")
}
