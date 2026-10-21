use std::sync::OnceLock;

use napi::{bindgen_prelude::*, JsValue, UnknownRef};
use napi_derive::napi;
use topk_rs::proto::v1::data::image;

/// @hideconstructor
/// Instances of the `Image` class are used to represent images in TopK.
/// Created using the [`image()`](https://docs.topk.io/sdk/topk-js/data#image) helper and returned for image fields by `get()`.
/// Image fields can't be selected in queries, use `get()` with the `_id`s returned from a query instead.
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
    pub(crate) data: image::Data,
    /// `data` as a JS `Buffer`, created on first access
    buffer: OnceLock<BufferRef>,
}

/// Reference to the JS `Buffer` returned from `Image.data`.
struct BufferRef(UnknownRef<false>);

// SAFETY: the reference is only read and released through an `Env`, which only exists on the
// JS thread. Dropping it on another thread doesn't call into the JS engine.
unsafe impl Sync for BufferRef {}

impl Image {
    pub(crate) fn new(data: image::Data) -> Self {
        Image {
            data,
            buffer: OnceLock::new(),
        }
    }
}

/// Clones only the image data, the cached `Buffer` belongs to the JS object.
impl Clone for Image {
    fn clone(&self) -> Self {
        Image::new(self.data.clone())
    }
}

impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len = self.data.bytes().len();
        write!(f, "Image({}, {len} bytes)", self.format())
    }
}

#[napi(namespace = "data")]
impl Image {
    /// Encoding format of the image.
    #[napi(getter, ts_return_type = "'jpeg' | 'png'")]
    pub fn format(&self) -> &'static str {
        match self.data {
            image::Data::Jpeg(_) => "jpeg",
            image::Data::Png(_) => "png",
        }
    }

    /// Encoded image bytes.
    ///
    /// The same `Buffer` is returned on every access, modifying it doesn't change the image.
    #[napi(getter)]
    pub fn data<'env>(&self, env: &'env Env) -> Result<BufferSlice<'env>> {
        if self.buffer.get().is_none() {
            let buffer = BufferSlice::copy_from(env, self.data.bytes())?;
            let buffer = unsafe { UnknownRef::from_napi_value(env.raw(), buffer.raw())? };
            let _ = self.buffer.set(BufferRef(buffer));
        }

        let buffer = self.buffer.get().unwrap().0.get_value(env)?;
        unsafe { BufferSlice::from_napi_value(env.raw(), buffer.raw()) }
    }

    /// @ignore
    #[napi]
    pub fn to_string(&self) -> String {
        format!("{self:?}")
    }
}

impl ObjectFinalize for Image {
    fn finalize(self, env: Env) -> Result<()> {
        match self.buffer.into_inner() {
            Some(BufferRef(buffer)) => buffer.unref(&env),
            None => Ok(()),
        }
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
