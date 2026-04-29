use std::collections::HashMap;

use crate::proto::ctx::v1::{
    ask_result::{Answer, Message, Search},
    AskResult, Fact, SearchResult,
};

impl AskResult {
    pub fn answer(facts: Vec<Fact>, refs: HashMap<String, SearchResult>) -> Self {
        Self {
            message: Some(Message::Answer(Answer { facts, refs })),
        }
    }

    pub fn search(query: String) -> Self {
        Self {
            message: Some(Message::Search(Search { query })),
        }
    }
}
