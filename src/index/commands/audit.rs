use chrono::{Datelike, DateTime, Local};
use crate::error::Error;
use crate::index::Index;
use ragit_api::audit::AuditRecord;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Audit {
    pub input_tokens: u64,
    pub output_tokens: u64,

    /// divide this by 1_000_000 to get dollars
    pub input_cost: u64,
    pub output_cost: u64,
}

impl Audit {
    pub fn is_empty(&self) -> bool {
        self.input_tokens == 0 && self.output_tokens == 0 && self.input_cost == 0 && self.output_cost == 0
    }
}

impl std::ops::AddAssign for Audit {
    fn add_assign(&mut self, other: Audit) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.input_cost += other.input_cost;
        self.output_cost += other.output_cost;
    }
}

impl Index {
    /// `rag audit`
    ///
    /// If `since` is set, it only counts record after the timestamp. It's a unix timestamp, usually acquired by `chrono::Local::now().timestamp()`
    pub fn audit(&self, since: Option<DateTime<Local>>) -> Result<HashMap<String, Audit>, Error> {
        let mut result = HashMap::new();
        let since = match since {
            Some(since) => format!("{:04}{:02}{:02}", since.year(), since.month(), since.day()),
            None => String::from("00000000"),
        };

        // TODO: it's not a good idea to hard-code all the keys...
        for key in [
            "create_chunk_from",
            "describe_image",
            "rerank_summary",
            "answer_query_with_chunks",
            "rephrase_multi_turn",
            "raw_request",
            "extract_keywords",
            "agent",
            "pdl",
        ] {
            let mut audit = Audit::default();

            match self.api_config.get_api_usage(&self.root_dir, key) {
                Ok(records) => {
                    for (date, AuditRecord { input_tokens, output_tokens, input_cost, output_cost }) in records.iter() {
                        if date >= &since {
                            audit.input_tokens += input_tokens;
                            audit.output_tokens += output_tokens;
                            audit.input_cost += input_cost;
                            audit.output_cost += output_cost;
                        }
                    }
                },
                Err(_) => {},
            }

            result.insert(key.to_string(), audit);
        }

        Ok(result)
    }
}
