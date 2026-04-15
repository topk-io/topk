use std::collections::HashMap;

use napi::bindgen_prelude::{Either3, *};
use napi_derive::napi;
use topk_rs::proto::v1::ctx::ask_result::Message;
use topk_rs::proto::v1::ctx::content::Data;

use crate::data::NativeValue;
use crate::error::TopkError;
use crate::expr::logical::LogicalExpression;

pub type AskResultEither = Either3<Answer, Search, Reason>;

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

/// A dataset selector for ask/search operations.
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

/// A search result from context search or ask references.
#[napi(object, object_from_js = false)]
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub doc_id: String,
    pub doc_type: String,
    pub dataset: String,
    pub content: Content,
    #[napi(ts_type = "Record<string, any>")]
    pub metadata: HashMap<String, NativeValue>,
}

impl TryFrom<topk_rs::proto::v1::ctx::SearchResult> for SearchResult {
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
        };

        Ok(SearchResult {
            doc_id: v.doc_id,
            doc_type: v.doc_type,
            dataset: v.dataset,
            content,
            metadata: v.metadata.into_iter().map(|(k, v)| (k, v.into())).collect(),
        })
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

/// An answer from the ask API containing facts and references.
#[napi(object, object_from_js = false)]
#[derive(Debug, Clone)]
pub struct Answer {
    pub facts: Vec<Fact>,
    pub refs: HashMap<String, SearchResult>,
}

/// A search step from the ask API containing an objective, facts, and references.
#[napi(object, object_from_js = false)]
#[derive(Debug, Clone)]
pub struct Search {
    pub objective: String,
    pub facts: Vec<Fact>,
    pub refs: HashMap<String, SearchResult>,
}

/// A reasoning step from the ask API.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct Reason {
    pub thought: String,
}

/// Converts a proto AskResult to Answer, returning an error for any other message type.
pub fn convert_ask_result_to_answer(
    result: topk_rs::proto::v1::ctx::AskResult,
) -> napi::Result<Answer> {
    let convert_refs = |refs: HashMap<String, topk_rs::proto::v1::ctx::SearchResult>| {
        refs.into_iter()
            .map(|(k, v)| SearchResult::try_from(v).map(|sr| (k, sr)))
            .collect::<napi::Result<HashMap<_, _>>>()
    };

    match result.message {
        Some(Message::Answer(fa)) => {
            let refs = convert_refs(fa.refs)?;
            Ok(Answer {
                facts: fa.facts.into_iter().map(Fact::from).collect(),
                refs,
            })
        }
        Some(_) => Err(napi::Error::from_reason(
            "ask: expected Answer but received a different message type",
        )),
        None => Err(napi::Error::from_reason("ask: result has no message")),
    }
}

/// Converts a proto AskResult into one of the three concrete JS types.
pub fn convert_ask_result(
    result: topk_rs::proto::v1::ctx::AskResult,
) -> napi::Result<AskResultEither> {
    let convert_refs = |refs: HashMap<String, topk_rs::proto::v1::ctx::SearchResult>| {
        refs.into_iter()
            .map(|(k, v)| SearchResult::try_from(v).map(|sr| (k, sr)))
            .collect::<napi::Result<HashMap<_, _>>>()
    };

    match result.message {
        Some(Message::Answer(fa)) => {
            let refs = convert_refs(fa.refs)?;
            Ok(Either3::A(Answer {
                facts: fa.facts.into_iter().map(Fact::from).collect(),
                refs,
            }))
        }
        Some(Message::Search(sq)) => {
            let refs = convert_refs(sq.refs)?;
            Ok(Either3::B(Search {
                objective: sq.objective,
                facts: sq.facts.into_iter().map(Fact::from).collect(),
                refs,
            }))
        }
        Some(Message::Reason(r)) => Ok(Either3::C(Reason { thought: r.thought })),
        None => Err(napi::Error::from_reason("AskResult has no message")),
    }
}
