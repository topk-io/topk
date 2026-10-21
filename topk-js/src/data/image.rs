use napi::bindgen_prelude::*;
use napi_derive::napi;
use topk_rs::proto::v1::data::image;

/// Image encoding format.
#[napi(string_enum = "lowercase", namespace = "data")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageFormat {
    Jpeg,
    Png,
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
#[napi(namespace = "data")]
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

#[napi(namespace = "data")]
impl Image {
    /// Encoding format of the image.
    #[napi(getter)]
    pub fn format(&self) -> ImageFormat {
        match &self.data {
            image::Data::Jpeg(_) => ImageFormat::Jpeg,
            image::Data::Png(_) => ImageFormat::Png,
        }
    }

    /// Encoded image bytes.
    #[napi(getter)]
    pub fn data(&self) -> Buffer {
        match &self.data {
            image::Data::Jpeg(data) | image::Data::Png(data) => data.to_vec().into(),
        }
    }

    /// @ignore
    #[napi]
    pub fn to_string(&self) -> String {
        format!("{self:?}")
    }
}

/// Encoded image bytes passed from JS as a `Buffer` or `Uint8Array`.
pub(crate) struct ImageBytes(pub(crate) Vec<u8>);

impl FromNapiValue for ImageBytes {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        Uint8Array::from_napi_value(env, value)
            .map(|data| ImageBytes(data.to_vec()))
            .map_err(|_| {
                napi::Error::from_reason("Invalid image data, must be `Buffer` or `Uint8Array`")
            })
    }
}
