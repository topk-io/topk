use bytes::Bytes;
use pyo3::{
    exceptions::PyImportError,
    prelude::*,
    types::{PyBytes, PyDict},
};
use topk_rs::proto::v1::data::image;

use crate::error::InvalidArgumentError;

/// PIL modes that can be written as PNG without a conversion.
const PNG_MODES: &[&str] = &["1", "L", "LA", "I", "I;16", "I;16B", "P", "RGB", "RGBA"];

#[pyclass(frozen, eq)]
#[derive(Clone, PartialEq)]
pub struct Image {
    pub(crate) data: image::Data,
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.data {
            image::Data::Jpeg(data) => write!(f, "Image(jpeg, {} bytes)", data.len()),
            image::Data::Png(data) => write!(f, "Image(png, {} bytes)", data.len()),
        }
    }
}

#[pymethods]
impl Image {
    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

impl Image {
    /// Create an image from encoded JPEG or PNG bytes.
    pub(crate) fn from_bytes(data: &[u8]) -> PyResult<Self> {
        image::Data::from_bytes(Bytes::copy_from_slice(data))
            .map(|data| Image { data })
            .ok_or_else(|| {
                InvalidArgumentError::new_err("Unsupported image format, expected JPEG or PNG data")
            })
    }

    /// Encode a PIL image. Returns `None` if `obj` is not a PIL image.
    ///
    /// Images decoded from JPEG are re-encoded as JPEG with the original quantization
    /// tables, everything else is encoded losslessly as PNG.
    pub(crate) fn from_pil(obj: &Bound<'_, PyAny>) -> PyResult<Option<Self>> {
        let py = obj.py();

        // An object can't be a PIL image unless Pillow has already been imported
        let modules = py
            .import("sys")?
            .getattr("modules")?
            .cast_into::<PyDict>()?;
        let Some(pil) = modules.get_item("PIL.Image")? else {
            return Ok(None);
        };
        if !obj.is_instance(&pil.getattr("Image")?)? {
            return Ok(None);
        }

        // Carry over the color profile and EXIF metadata (e.g. orientation)
        let kwargs = PyDict::new(py);
        let info = obj.getattr("info")?.cast_into::<PyDict>()?;
        for key in ["icc_profile", "exif"] {
            if let Some(value) = info.get_item(key)? {
                kwargs.set_item(key, value)?;
            }
        }

        let format: Option<String> = obj.getattr("format")?.extract()?;
        let (obj, format) = match format.as_deref() {
            Some("JPEG") => {
                kwargs.set_item("quality", "keep")?;
                (obj.clone(), "JPEG")
            }
            // Multi-picture JPEG, Pillow can only "keep" the quality of plain JPEGs
            Some("MPO") => {
                kwargs.set_item("quality", 95)?;
                (obj.clone(), "JPEG")
            }
            _ => {
                let mode: String = obj.getattr("mode")?.extract()?;
                if PNG_MODES.contains(&mode.as_str()) {
                    (obj.clone(), "PNG")
                } else {
                    let bands: Vec<String> = obj.call_method0("getbands")?.extract()?;
                    let mode = match bands.iter().any(|b| b == "A" || b == "a") {
                        true => "RGBA",
                        false => "RGB",
                    };
                    (obj.call_method1("convert", (mode,))?, "PNG")
                }
            }
        };

        let buf = py.import("io")?.getattr("BytesIO")?.call0()?;
        obj.call_method("save", (&buf, format), Some(&kwargs))?;
        let encoded = buf.call_method0("getvalue")?;
        let encoded = Bytes::copy_from_slice(encoded.cast::<PyBytes>()?.as_bytes());

        Ok(Some(Image {
            data: match format {
                "JPEG" => image::Data::Jpeg(encoded),
                _ => image::Data::Png(encoded),
            },
        }))
    }

    /// Decode into a PIL image.
    pub(crate) fn to_pil<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let pil = py.import("PIL.Image").map_err(|e| {
            let err = PyImportError::new_err(
                "Pillow is required to read image values, install it with `pip install pillow`",
            );
            err.set_cause(py, Some(e));
            err
        })?;

        let (data, format) = match &self.data {
            image::Data::Jpeg(data) => (data, "JPEG"),
            image::Data::Png(data) => (data, "PNG"),
        };
        let buf = py
            .import("io")?
            .getattr("BytesIO")?
            .call1((PyBytes::new(py, data),))?;

        let kwargs = PyDict::new(py);
        kwargs.set_item("formats", [format])?;
        pil.call_method("open", (buf,), Some(&kwargs))
    }
}
