use std::path::PathBuf;

use bytes::Bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use topk_rs::proto::v1::ctx::file::InputFile;

use crate::error::RustError;

pub enum FileOrFileLike {
    Path(String),
    Tuple(String, Bytes, String),
}

impl FromPyObject<'_, '_> for FileOrFileLike {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
        // Check if it's a tuple first (tuples can be strings too, so check before string)
        if let Ok(tuple) = obj.cast::<pyo3::types::PyTuple>() {
            match tuple.len() {
                3 => {
                    let filename = tuple.get_item(0)?.extract::<String>()?;
                    let contents_bound = tuple.get_item(1)?;
                    let contents = contents_bound.cast::<PyBytes>()?;
                    let mime_type = tuple.get_item(2)?.extract::<String>()?;

                    return Ok(FileOrFileLike::Tuple(
                        filename,
                        Bytes::from(contents.as_bytes().to_vec()),
                        mime_type,
                    ));
                }
                _ => {
                    return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                        "Expected tuple of (filename, contents, mime_type)",
                    ));
                }
            }
        }

        // Check if it's a PathBuf
        if let Ok(path) = obj.extract::<PathBuf>() {
            return Ok(FileOrFileLike::Path(path.to_string_lossy().to_string()));
        }

        // Check if it's a string
        if let Ok(string) = obj.extract::<String>() {
            return Ok(FileOrFileLike::Path(string));
        }

        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Expected PathLike or tuple of (filename, contents, mime_type)",
        ))
    }
}

impl TryFrom<FileOrFileLike> for InputFile {
    type Error = PyErr;

    fn try_from(value: FileOrFileLike) -> PyResult<Self> {
        match value {
            FileOrFileLike::Path(path) => {
                InputFile::from_path(path).map_err(|e| RustError(e).into())
            }
            FileOrFileLike::Tuple(filename, contents, mime_type) => {
                InputFile::from_bytes(filename, contents, mime_type)
                    .map_err(|e| RustError(e).into())
            }
        }
    }
}
