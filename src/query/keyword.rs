use crate::Index;
use crate::error::Error;
use crate::index::tfidf::tokenize;
use ragit_api::Request;
use ragit_pdl::{
    Pdl,
    escape_pdl_tokens,
    parse_pdl,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Keywords {
    // important keywords and less important keywords
    pub keywords: Vec<String>,
    pub extra: Vec<String>,
}

impl Keywords {
    pub fn from_raw(keywords: Vec<String>) -> Self {
        Keywords {
            keywords,
            extra: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.keywords.is_empty() && self.extra.is_empty()
    }

    /// `keywords` is `weight` times more important than `extrat`.
    pub fn with_weights(&self, weight: f32) -> Vec<(String, f32)> {
        self.keywords.iter().map(
            |keyword| (keyword.to_string(), weight / (weight + 1.0))
        ).chain(self.extra.iter().map(
            |extra| (extra.to_string(), 1.0 / (weight + 1.0))
        )).collect()
    }

    /// You don't have to call this function unless you want to see the internals.
    /// `TfidfState` will call this method at right timing. If you have keywords
    /// to search but don't know what to do, just run `Keywords::from_raw(keywords)`
    /// and pass it to  `TfidfState`. If you have only 1 `String`, not `Vec<String>`,
    /// `Keywords::from_raw(vec![keyword])` is fine.
    pub fn tokenize(&self) -> HashMap<String, f32> {  // HashMap<Token, weight>
        let mut tokens = HashMap::new();

        for (keyword, weight) in self.with_weights(4.0) {
            for token in tokenize(&keyword) {
                match tokens.get_mut(&token) {
                    Some(w) => {
                        *w += weight;
                    },
                    None => {
                        tokens.insert(token, weight);
                    },
                }
            }
        }

        tokens
    }
}

impl Index {
    pub async fn extract_keywords(
        &self,
        query: &str,
    ) -> Result<Keywords, Error> {
        let mut context = tera::Context::new();
        context.insert("query", &escape_pdl_tokens(&query));

        let Pdl { messages, schema } = parse_pdl(
            &self.get_prompt("extract_keyword")?,  // TODO: function name and prompt name are not matching
            &context,
            "/",  // TODO: `<|media|>` is not supported for this prompt
            true,
            true,
        )?;

        let request = Request {
            messages,
            model: self.get_model_by_name(&self.api_config.model)?,
            frequency_penalty: None,
            max_tokens: None,
            max_retry: self.api_config.max_retry,
            sleep_between_retries: self.api_config.sleep_between_retries,
            timeout: self.api_config.timeout,
            temperature: None,
            record_api_usage_at: self.api_config.dump_api_usage_at(&self.root_dir, "extract_keywords"),
            dump_pdl_at: self.api_config.create_pdl_path(&self.root_dir, "extract_keywords"),
            dump_json_at: self.api_config.dump_log_at(&self.root_dir),
            schema,
            schema_max_try: 3,
        };
        Ok(request.send_and_validate::<Keywords>(Keywords::from_raw(query.split(" ").map(|k| k.to_string()).collect())).await?)
    }
}
