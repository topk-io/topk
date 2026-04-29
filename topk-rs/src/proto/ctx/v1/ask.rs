use std::collections::HashMap;

use crate::proto::ctx::v1::{
    ask_result::{Answer, Message, Progress},
    AskResult, Fact, SearchResult,
};

impl AskResult {
    pub fn answer(facts: Vec<Fact>, refs: HashMap<String, SearchResult>) -> Self {
        Self {
            message: Some(Message::Answer(Answer { facts, refs })),
        }
    }

    pub fn progress(update: String) -> Self {
        Self {
            message: Some(Message::Progress(Progress { update })),
        }
    }
}
