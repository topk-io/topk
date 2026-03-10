use std::path::PathBuf;

use topk_rs::proto::v1::ctx::file::InputFile;

pub mod books;
pub mod multi_vec;
pub mod semantic;

#[allow(dead_code)]
pub fn test_pdf() -> InputFile {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("utils")
        .join("dataset")
        .join("pdfko.pdf");

    InputFile::from_path(&path).expect("could not create InputFile from path")
}
