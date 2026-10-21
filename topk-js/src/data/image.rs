use napi::{bindgen_prelude::*, JsValue, UnknownRef};
use napi_derive::napi;
use topk_rs::proto::v1::data::image;

const JPEG_SIGNATURE: &[u8] = b"\xFF\xD8\xFF";
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1A\n";

/// Image encoding format.
#[napi(string_enum = "lowercase", namespace = "data")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageFormat {
    Jpeg,
    Png,
}

impl ImageFormat {
    /// Detect the format from the file signature.
    pub(crate) fn detect(data: &[u8]) -> Option<Self> {
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
}

/// Encoded image held in a document [`Value`](crate::data::Value).
#[derive(Clone, PartialEq)]
pub struct ImageValue(pub(crate) image::Data);

impl std::fmt::Debug for ImageValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (format, data) = self.parts();
        write!(f, "{}, {} bytes", format.as_str(), data.len())
    }
}

impl ImageValue {
    fn parts(&self) -> (ImageFormat, &[u8]) {
        match &self.0 {
            image::Data::Jpeg(data) => (ImageFormat::Jpeg, data),
            image::Data::Png(data) => (ImageFormat::Png, data),
        }
    }
}

impl ToNapiValue for ImageValue {
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> Result<sys::napi_value> {
        let (format, data) = val.parts();
        let image = Image::new(&Env::from_raw(env), format, data)?;
        Image::to_napi_value(env, image)
    }
}

/// @hideconstructor
/// Instances of the `Image` class are used to represent images in TopK.
/// Created using the [`image()`](https://docs.topk.io/sdk/topk-js/data#image) helper and returned for image fields in query results.
///
/// Example:
///
/// ```javascript
/// import sharp from "sharp";
///
/// const docs = await client.collection("books").get(["gatsby"]);
/// const cover = docs["gatsby"].cover;
///
/// cover.format // "jpeg"
/// await sharp(cover.data).resize(64).toFile("thumbnail.png");
/// ```
#[napi(namespace = "data", custom_finalize)]
pub struct Image {
    format: ImageFormat,
    /// JS `Buffer` with the encoded bytes, returned as-is from the `data` getter
    data: UnknownRef<false>,
}

impl Image {
    /// Create an image backed by a new JS `Buffer` holding a copy of `data`.
    pub(crate) fn new(env: &Env, format: ImageFormat, data: &[u8]) -> Result<Self> {
        let buffer = BufferSlice::copy_from(env, data)?;
        let data = unsafe { UnknownRef::from_napi_value(env.raw(), buffer.raw())? };
        Ok(Image { format, data })
    }

    fn buffer<'env>(&self, env: &'env Env) -> Result<BufferSlice<'env>> {
        let buffer = self.data.get_value(env)?;
        unsafe { BufferSlice::from_napi_value(env.raw(), buffer.raw()) }
    }

    /// Copy the encoded bytes into a document value.
    pub(crate) fn to_value(&self, env: &Env) -> Result<ImageValue> {
        let data = self.buffer(env)?.to_vec().into();
        Ok(ImageValue(match self.format {
            ImageFormat::Jpeg => image::Data::Jpeg(data),
            ImageFormat::Png => image::Data::Png(data),
        }))
    }
}

#[napi(namespace = "data")]
impl Image {
    /// Encoding format of the image.
    #[napi(getter)]
    pub fn format(&self) -> ImageFormat {
        self.format
    }

    /// Encoded image bytes.
    #[napi(getter)]
    pub fn data<'env>(&self, env: &'env Env) -> Result<BufferSlice<'env>> {
        self.buffer(env)
    }

    /// @ignore
    #[napi]
    pub fn to_string(&self, env: &Env) -> Result<String> {
        Ok(format!(
            "Image({}, {} bytes)",
            self.format.as_str(),
            self.buffer(env)?.len()
        ))
    }
}

impl ObjectFinalize for Image {
    fn finalize(self, env: Env) -> Result<()> {
        self.data.unref(&env)
    }
}

/// Encoded image bytes passed from JS as a `Buffer` or `Uint8Array`.
pub(crate) struct ImageBytes(pub(crate) Uint8Array);

impl FromNapiValue for ImageBytes {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        Uint8Array::from_napi_value(env, value)
            .map(ImageBytes)
            .map_err(|_| {
                napi::Error::from_reason("Invalid image data, must be `Buffer` or `Uint8Array`")
            })
    }
}
