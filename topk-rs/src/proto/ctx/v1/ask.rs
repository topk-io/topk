use std::collections::HashMap;

use crate::proto::ctx::v1::{
    ask_response_message::{FinalAnswer, Message, Reason, SubQuery},
    AskResponseMessage, Fact, SearchResult,
};

impl AskResponseMessage {
    pub fn final_answer(facts: Vec<Fact>, sources: HashMap<String, SearchResult>) -> Self {
        Self {
            message: Some(Message::FinalAnswer(FinalAnswer { facts, sources })),
        }
    }

    pub fn sub_query(
        objective: String,
        facts: Vec<Fact>,
        sources: HashMap<String, SearchResult>,
    ) -> Self {
        Self {
            message: Some(Message::SubQuery(SubQuery {
                objective,
                facts,
                sources,
            })),
        }
    }

    pub fn reason(thought: String) -> Self {
        Self {
            message: Some(Message::Reason(Reason { thought })),
        }
    }
}
