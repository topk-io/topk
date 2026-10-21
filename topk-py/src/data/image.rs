use std::ffi::CString;

use bytes::Bytes;
use pyo3::{
    exceptions::{PyException, PyImportError, PyUserWarning, PyValueError},
    prelude::*,
    types::{PyBytes, PyDict},
};
use topk_rs::proto::v1::data::image;

use crate::error::InvalidArgumentError;

/// PIL modes that can be written as PNG without a conversion.
const PNG_MODES: &[&str] = &["1", "L", "LA", "I", "I;16", "I;16B", "P", "RGB", "RGBA"];

const JPEG_SIGNATURE: &[u8] = b"\xFF\xD8\xFF";
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1A\n";

#[derive(Debug, Clone, Copy, PartialEq)]
enum ImageFormat {
    Jpeg,
    Png,
}

impl ImageFormat {
    /// Detect the format from the file signature.
    fn detect(data: &[u8]) -> Option<Self> {
        if data.starts_with(JPEG_SIGNATURE) {
            Some(ImageFormat::Jpeg)
        } else if data.starts_with(PNG_SIGNATURE) {
            Some(ImageFormat::Png)
        } else {
            None
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "jpeg",
            ImageFormat::Png => "png",
        }
    }

    /// Format name used by PIL.
    fn pil_name(self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "JPEG",
            ImageFormat::Png => "PNG",
        }
    }
}

/// Encoded image held in a document `Value`.
#[derive(Clone, PartialEq)]
pub struct ImageValue(pub(crate) image::Data);

impl std::fmt::Debug for ImageValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (format, data) = self.parts();
        write!(f, "{}, {} bytes", format.as_str(), data.len())
    }
}

impl ImageValue {
    fn parts(&self) -> (ImageFormat, &Bytes) {
        match &self.0 {
            image::Data::Jpeg(data) => (ImageFormat::Jpeg, data),
            image::Data::Png(data) => (ImageFormat::Png, data),
        }
    }

    /// Convert into a PIL image. If Pillow is not installed or can't decode the data,
    /// warn and return an `Image` instead, so one image can't fail the whole read.
    pub(crate) fn into_py<'py>(self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let (format, data) = self.parts();
        let data = PyBytes::new(py, data);

        match open_pil(py, &data, format) {
            Ok(image) => Ok(image),
            Err(err) if err.is_instance_of::<PyException>(py) => {
                let message = if err.is_instance_of::<PyImportError>(py) {
                    "Pillow is not installed, returning image values as `topk_sdk.data.Image`. \
                     Install it with `pip install pillow` to get PIL images."
                        .to_string()
                } else {
                    format!(
                        "Cannot decode image value, returning it as `topk_sdk.data.Image`: {err}"
                    )
                };
                let message =
                    CString::new(message).map_err(|e| PyValueError::new_err(e.to_string()))?;
                PyErr::warn(py, py.get_type::<PyUserWarning>().as_any(), &message, 1)?;

                let image = Image {
                    format,
                    data: data.unbind(),
                };
                Ok(Bound::new(py, image)?.into_any())
            }
            Err(err) => Err(err),
        }
    }
}

fn open_pil<'py>(
    py: Python<'py>,
    data: &Bound<'py, PyBytes>,
    format: ImageFormat,
) -> PyResult<Bound<'py, PyAny>> {
    let pil = py.import("PIL.Image")?;
    let buf = py.import("io")?.getattr("BytesIO")?.call1((data,))?;

    let kwargs = PyDict::new(py);
    kwargs.set_item("formats", [format.pil_name()])?;
    pil.call_method("open", (buf,), Some(&kwargs))
}

#[pyclass(frozen)]
pub struct Image {
    format: ImageFormat,
    data: Py<PyBytes>,
}

#[pymethods]
impl Image {
    /// Encoding format of the image, `jpeg` or `png`.
    #[getter]
    fn format(&self) -> &'static str {
        self.format.as_str()
    }

    /// Encoded image bytes.
    #[getter]
    fn data(&self, py: Python<'_>) -> Py<PyBytes> {
        self.data.clone_ref(py)
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        format!(
            "Image({}, {} bytes)",
            self.format.as_str(),
            self.data.bind(py).as_bytes().len()
        )
    }
}

impl Image {
    /// Create an image from encoded JPEG or PNG bytes.
    pub(crate) fn from_bytes(data: &Bound<'_, PyBytes>) -> PyResult<Self> {
        let format = ImageFormat::detect(data.as_bytes()).ok_or_else(|| {
            InvalidArgumentError::new_err("Unsupported image format, expected JPEG or PNG data")
        })?;

        Ok(Image {
            format,
            data: data.clone().unbind(),
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
        let pil = match modules.get_item("PIL.Image")? {
            Some(pil) if !pil.is_none() => pil,
            _ => return Ok(None),
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
                (obj.clone(), ImageFormat::Jpeg)
            }
            // Multi-picture JPEG, Pillow can only "keep" the quality of plain JPEGs
            Some("MPO") => {
                kwargs.set_item("quality", 95)?;
                (obj.clone(), ImageFormat::Jpeg)
            }
            _ => {
                let mode: String = obj.getattr("mode")?.extract()?;
                if PNG_MODES.contains(&mode.as_str()) {
                    (obj.clone(), ImageFormat::Png)
                } else {
                    let bands: Vec<String> = obj.call_method0("getbands")?.extract()?;
                    let mode = match bands.iter().any(|b| b == "A" || b == "a") {
                        true => "RGBA",
                        false => "RGB",
                    };
                    (obj.call_method1("convert", (mode,))?, ImageFormat::Png)
                }
            }
        };

        let buf = py.import("io")?.getattr("BytesIO")?.call0()?;
        obj.call_method("save", (&buf, format.pil_name()), Some(&kwargs))?;
        let data = buf.call_method0("getvalue")?.cast_into::<PyBytes>()?;

        Ok(Some(Image {
            format,
            data: data.unbind(),
        }))
    }

    /// Copy the encoded bytes into a document value.
    pub(crate) fn to_value(&self, py: Python<'_>) -> ImageValue {
        let data = Bytes::copy_from_slice(self.data.bind(py).as_bytes());
        ImageValue(match self.format {
            ImageFormat::Jpeg => image::Data::Jpeg(data),
            ImageFormat::Png => image::Data::Png(data),
        })
    }
}
