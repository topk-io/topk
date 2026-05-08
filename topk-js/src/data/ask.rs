use std::collections::HashMap;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use topk_rs::proto::v1::ctx::content::Data;

use crate::data::NativeValue;
use crate::expr::logical::LogicalExpression;

/// Mode for ask operations.
#[napi(string_enum = "lowercase")]
#[derive(Debug, Clone)]
pub enum Mode {
    Auto,
    Summarize,
    Research,
}

impl From<Mode> for topk_rs::proto::v1::ctx::Mode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Auto => topk_rs::proto::v1::ctx::Mode::Auto,
            Mode::Summarize => topk_rs::proto::v1::ctx::Mode::Summarize,
            Mode::Research => topk_rs::proto::v1::ctx::Mode::Research,
        }
    }
}

/// Represents a dataset with an optional filter.
#[derive(Debug, Clone)]
pub struct Source {
    pub dataset: String,
    pub filter: Option<LogicalExpression>,
}

impl From<Source> for topk_rs::proto::v1::ctx::Source {
    fn from(source: Source) -> Self {
        topk_rs::proto::v1::ctx::Source {
            dataset: source.dataset,
            filter: source.filter.map(|f| f.into()),
        }
    }
}

impl FromNapiValue for Source {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        if let Ok(dataset) = String::from_napi_value(env, value) {
            return Ok(Source {
                dataset,
                filter: None,
            });
        }

        let obj = Object::from_napi_value(env, value)?;
        let dataset: String = obj
            .get("dataset")?
            .ok_or_else(|| napi::Error::from_reason("Source object must have 'dataset' field"))?;

        let filter: Option<LogicalExpression> = if obj.has_named_property("filter")? {
            let filter_val = obj.get_named_property_unchecked::<Unknown>("filter")?;
            let raw = Unknown::to_napi_value(env, filter_val)?;
            let mut value_type: i32 = 0;
            napi::sys::napi_typeof(env, raw, &mut value_type);
            if value_type == napi::sys::ValueType::napi_undefined
                || value_type == napi::sys::ValueType::napi_null
            {
                None
            } else {
                Some(LogicalExpression::from_napi_ref(env, raw)?.clone())
            }
        } else {
            None
        };

        Ok(Source { dataset, filter })
    }
}

/// Text chunk content.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct Chunk {
    pub text: String,
    pub doc_pages: Vec<u32>,
}

/// Image content.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct Image {
    pub data: Vec<u8>,
    pub mime_type: String,
}

/// Page content with optional image.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct Page {
    pub page_number: u32,
    pub image: Option<Image>,
}

/// Content in a search result. One of chunk, page, or image.
#[napi(object, object_from_js = false)]
#[derive(Debug, Clone)]
pub struct Content {
    #[napi(ts_type = "\"chunk\" | \"page\" | \"image\"")]
    pub r#type: String,
    pub data: Either3<Chunk, Page, Image>,
}

/// Represents a search result in an ask response.
#[napi(object, object_from_js = false)]
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub doc_id: String,
    pub doc_type: String,
    pub doc_name: String,
    pub dataset: String,
    pub content_id: String,
    pub content: Option<Content>,
    #[napi(ts_type = "Record<string, any>")]
    pub metadata: HashMap<String, NativeValue>,
}

impl TryFrom<topk_rs::proto::v1::ctx::SearchResult> for SearchResult {
    type Error = napi::Error;

    fn try_from(v: topk_rs::proto::v1::ctx::SearchResult) -> Result<Self> {
        let content = match v.content {
            None => None,
            Some(content) => Some(
                match content
                    .data
                    .ok_or_else(|| napi::Error::from_reason(topk_rs::Error::InvalidProto.to_string()))?
                {
                    Data::Chunk(chunk) => Content {
                        r#type: "chunk".to_string(),
                        data: Either3::A(Chunk {
                            text: chunk.text,
                            doc_pages: chunk.doc_pages,
                        }),
                    },
                    Data::Page(page) => Content {
                        r#type: "page".to_string(),
                        data: Either3::B(Page {
                            page_number: page.page_number,
                            image: page.image.map(|img| Image {
                                data: img.data.to_vec(),
                                mime_type: img.mime_type,
                            }),
                        }),
                    },
                    Data::Image(img) => Content {
                        r#type: "image".to_string(),
                        data: Either3::C(Image {
                            data: img.data.to_vec(),
                            mime_type: img.mime_type,
                        }),
                    },
                })
        };

        Ok(SearchResult {
            doc_id: v.doc_id,
            doc_type: v.doc_type,
            doc_name: v.doc_name,
            dataset: v.dataset,
            content_id: v.content_id,
            content,
            metadata: v.metadata.into_iter().map(|(k, v)| (k, v.into())).collect(),
        })
    }
}

/// Represents a fact in an ask response.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct Fact {
    pub fact: String,
    pub ref_ids: Vec<String>,
}

impl From<topk_rs::proto::v1::ctx::Fact> for Fact {
    fn from(f: topk_rs::proto::v1::ctx::Fact) -> Self {
        Fact {
            fact: f.fact,
            ref_ids: f.ref_ids,
        }
    }
}

/// Represents a final answer in an ask response.
#[napi(object, object_from_js = false)]
#[derive(Debug, Clone)]
pub struct Answer {
    pub facts: Vec<Fact>,
    pub refs: HashMap<String, SearchResult>,
    pub confidence: f32,
}

/// Represents a progress update in an ask response.
#[napi(object, object_from_js = false)]
#[derive(Debug, Clone)]
pub struct Progress {
    pub update: String,
}
