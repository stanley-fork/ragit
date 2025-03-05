use crate::error::Error;
use crate::index::Index;
use ragit_api::record::Record;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Audit {
    pub input_tokens: u64,
    pub output_tokens: u64,

    /// divide this by 1_000_000_000 to get dollars
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
    pub fn audit(&self, since: Option<u64>) -> Result<HashMap<String, Audit>, Error> {
        let mut result = HashMap::new();
        let since = since.unwrap_or(0);

        // TODO: it's not a good idea to hard-code all the keys...
        for key in [
            "create_chunk_from",
            "describe_image",
            "rerank_summary",
            "answer_query_with_chunks",
            "rephrase_multi_turn",
            "raw_request",
            "extract_keywords",
            "summary_chunks",
        ] {
            let mut audit = Audit::default();

            match self.api_config.get_api_usage(&self.root_dir, key) {
                Ok(records) => {
                    for Record { input, output, input_weight, output_weight, time } in records.iter() {
                        if *time >= since {
                            audit.input_tokens += input;
                            audit.output_tokens += output;
                            audit.input_cost += input * input_weight;
                            audit.output_cost += output * output_weight;
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
