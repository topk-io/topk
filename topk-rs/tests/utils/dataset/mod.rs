use std::path::PathBuf;

use topk_rs::proto::v1::ctx::file::InputFile;

pub mod books;
pub mod multi_vec;
pub mod semantic;

#[allow(dead_code)]
pub fn test_pdf() -> InputFile {
    test_file("pdfko.pdf")
}

#[allow(dead_code)]
pub fn test_file(name: &str) -> InputFile {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests")
        .join(name);

    InputFile::from_path(&path).expect("could not create InputFile from path")
}
