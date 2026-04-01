use std::collections::HashMap;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use topk_rs::proto::v1::ctx::ask_result::Message;
use topk_rs::proto::v1::ctx::content::Data;

use crate::data::NativeValue;
use crate::error::TopkError;
use crate::expr::logical::LogicalExpression;
use crate::utils::{js_object, js_set};

/// Mode for ask operations.
#[napi(string_enum = "lowercase")]
#[derive(Debug, Clone)]
pub enum Mode {
    Auto,
    Summarize,
    Reason,
    DeepResearch,
}

impl From<Mode> for topk_rs::proto::v1::ctx::Mode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Auto => topk_rs::proto::v1::ctx::Mode::Auto,
            Mode::Summarize => topk_rs::proto::v1::ctx::Mode::Summarize,
            Mode::Reason => topk_rs::proto::v1::ctx::Mode::Reason,
            Mode::DeepResearch => topk_rs::proto::v1::ctx::Mode::DeepResearch,
        }
    }
}

/// A source dataset for ask/search operations.
/// Accepts a string (dataset name) or `{ dataset: string, filter?: LogicalExpression }`.
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

impl ToNapiValue for Source {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        let obj = js_object(env)?;
        js_set(env, obj, "dataset", val.dataset)?;
        Ok(obj)
    }
}

/// A fact extracted from context.
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
pub struct ImageContent {
    pub data: Vec<u8>,
    pub mime_type: String,
}

/// Page content with optional image.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct Page {
    pub page_number: u32,
    pub image: Option<ImageContent>,
}

/// Content from a search result. Has a `type` field ("chunk", "page", or "image")
/// and a corresponding data field.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct Content {
    /// "chunk", "page", or "image"
    pub r#type: String,
    pub chunk: Option<Chunk>,
    pub page: Option<Page>,
    pub image: Option<ImageContent>,
}

/// A search result from context search or ask references.
#[derive(Debug, Clone)]
pub struct ContextSearchResult {
    pub doc_id: String,
    pub doc_type: String,
    pub dataset: String,
    pub content: Content,
    pub metadata: HashMap<String, NativeValue>,
}

impl TryFrom<topk_rs::proto::v1::ctx::SearchResult> for ContextSearchResult {
    type Error = napi::Error;

    fn try_from(mut v: topk_rs::proto::v1::ctx::SearchResult) -> Result<Self> {
        let content_data = v
            .content
            .take()
            .ok_or_else(|| TopkError::from(topk_rs::Error::InvalidProto))?
            .data
            .take()
            .ok_or_else(|| TopkError::from(topk_rs::Error::InvalidProto))?;

        let content = match content_data {
            Data::Chunk(chunk) => Content {
                r#type: "chunk".to_string(),
                chunk: Some(Chunk {
                    text: chunk.text,
                    doc_pages: chunk.doc_pages,
                }),
                page: None,
                image: None,
            },
            Data::Page(page) => Content {
                r#type: "page".to_string(),
                chunk: None,
                page: Some(Page {
                    page_number: page.page_number,
                    image: page.image.map(|img| ImageContent {
                        data: img.data.to_vec(),
                        mime_type: img.mime_type,
                    }),
                }),
                image: None,
            },
            Data::Image(img) => Content {
                r#type: "image".to_string(),
                chunk: None,
                page: None,
                image: Some(ImageContent {
                    data: img.data.to_vec(),
                    mime_type: img.mime_type,
                }),
            },
        };

        Ok(ContextSearchResult {
            doc_id: v.doc_id,
            doc_type: v.doc_type,
            dataset: v.dataset,
            content,
            metadata: v
                .metadata
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
        })
    }
}

impl ToNapiValue for ContextSearchResult {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        let obj = js_object(env)?;
        js_set(env, obj, "docId", val.doc_id)?;
        js_set(env, obj, "docType", val.doc_type)?;
        js_set(env, obj, "dataset", val.dataset)?;
        js_set(env, obj, "content", val.content)?;

        let meta = js_object(env)?;
        for (k, v) in val.metadata {
            js_set(env, meta, &k, v)?;
        }
        let key = std::ffi::CString::new("metadata").unwrap();
        napi::check_status!(napi::sys::napi_set_named_property(
            env,
            obj,
            key.as_ptr(),
            meta
        ))?;

        Ok(obj)
    }
}

/// An answer from the ask API containing facts and references.
#[derive(Debug, Clone)]
pub struct Answer {
    pub facts: Vec<Fact>,
    pub refs: HashMap<String, ContextSearchResult>,
}

/// A search step from the ask API containing an objective, facts, and references.
#[derive(Debug, Clone)]
pub struct SearchStep {
    pub objective: String,
    pub facts: Vec<Fact>,
    pub refs: HashMap<String, ContextSearchResult>,
}

/// A reasoning step from the ask API.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct Reason {
    pub thought: String,
}

/// Result from the ask API. Exactly one of `answer`, `search`, or `reason` will be set,
/// indicated by the `type` field.
#[derive(Debug, Clone)]
pub struct AskResult {
    pub r#type: String,
    pub answer: Option<Answer>,
    pub search: Option<SearchStep>,
    pub reason: Option<Reason>,
}

impl TryFrom<Message> for AskResult {
    type Error = napi::Error;

    fn try_from(msg: Message) -> Result<Self> {
        match msg {
            Message::Answer(fa) => {
                let refs = fa
                    .refs
                    .into_iter()
                    .map(|(k, v)| ContextSearchResult::try_from(v).map(|sr| (k, sr)))
                    .collect::<Result<HashMap<_, _>>>()?;

                Ok(AskResult {
                    r#type: "answer".to_string(),
                    answer: Some(Answer {
                        facts: fa.facts.into_iter().map(Fact::from).collect(),
                        refs,
                    }),
                    search: None,
                    reason: None,
                })
            }
            Message::Search(sq) => {
                let refs = sq
                    .refs
                    .into_iter()
                    .map(|(k, v)| ContextSearchResult::try_from(v).map(|sr| (k, sr)))
                    .collect::<Result<HashMap<_, _>>>()?;

                Ok(AskResult {
                    r#type: "search".to_string(),
                    answer: None,
                    search: Some(SearchStep {
                        objective: sq.objective,
                        facts: sq.facts.into_iter().map(Fact::from).collect(),
                        refs,
                    }),
                    reason: None,
                })
            }
            Message::Reason(r) => Ok(AskResult {
                r#type: "reason".to_string(),
                answer: None,
                search: None,
                reason: Some(Reason {
                    thought: r.thought,
                }),
            }),
        }
    }
}

unsafe fn refs_to_napi(
    env: napi::sys::napi_env,
    refs: HashMap<String, ContextSearchResult>,
) -> napi::Result<napi::sys::napi_value> {
    let obj = js_object(env)?;
    for (k, v) in refs {
        js_set(env, obj, &k, v)?;
    }
    Ok(obj)
}

impl ToNapiValue for Answer {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        let obj = js_object(env)?;
        js_set(env, obj, "facts", val.facts)?;
        let refs = refs_to_napi(env, val.refs)?;
        let key = std::ffi::CString::new("refs").unwrap();
        napi::check_status!(napi::sys::napi_set_named_property(
            env,
            obj,
            key.as_ptr(),
            refs
        ))?;
        Ok(obj)
    }
}

impl ToNapiValue for SearchStep {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        let obj = js_object(env)?;
        js_set(env, obj, "objective", val.objective)?;
        js_set(env, obj, "facts", val.facts)?;
        let refs = refs_to_napi(env, val.refs)?;
        let key = std::ffi::CString::new("refs").unwrap();
        napi::check_status!(napi::sys::napi_set_named_property(
            env,
            obj,
            key.as_ptr(),
            refs
        ))?;
        Ok(obj)
    }
}

impl ToNapiValue for AskResult {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        let obj = js_object(env)?;
        js_set(env, obj, "type", val.r#type)?;
        if let Some(answer) = val.answer {
            js_set(env, obj, "answer", answer)?;
        }
        if let Some(search) = val.search {
            js_set(env, obj, "search", search)?;
        }
        if let Some(reason) = val.reason {
            js_set(env, obj, "reason", reason)?;
        }
        Ok(obj)
    }
}
