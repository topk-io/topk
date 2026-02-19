use std::collections::HashMap;

use crate::proto::ctx::v1::{
    ask_response_message::{FinalAnswer, Message, Reason, Search},
    AskResponseMessage, Fact, SearchResult,
};

impl AskResponseMessage {
    pub fn final_answer(facts: Vec<Fact>, refs: HashMap<String, SearchResult>) -> Self {
        Self {
            message: Some(Message::FinalAnswer(FinalAnswer { facts, refs })),
        }
    }

    pub fn search(
        objective: String,
        facts: Vec<Fact>,
        refs: HashMap<String, SearchResult>,
    ) -> Self {
        Self {
            message: Some(Message::Search(Search {
                objective,
                facts,
                refs,
            })),
        }
    }

    pub fn reason(thought: String) -> Self {
        Self {
            message: Some(Message::Reason(Reason { thought })),
        }
    }
}
