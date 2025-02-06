use crate::Index;
use crate::error::Error;
use crate::index::tfidf::tokenize;
use ragit_api::{
    RecordAt,
    Request,
};
use ragit_pdl::{
    Pdl,
    escape_pdl_tokens,
    parse_pdl,
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq)]
pub struct Keywords {
    // important keywords and less important keywords
    keywords: Vec<String>,
    extra: Vec<String>,
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

    // keywords can be any string. it can be fed by user or ai
    // keywords are tokenized and deduplicated before tfidf
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

pub async fn extract_keywords(
    index: &Index,
    query: &str,
) -> Result<Keywords, Error> {
    let mut context = tera::Context::new();
    context.insert("query", &escape_pdl_tokens(&query));

    let Pdl { messages, schema } = parse_pdl(
        &index.get_prompt("extract_keyword")?,  // TODO: function name and prompt name are not matching
        &context,
        "/",  // TODO: `<|media|>` is not supported for this prompt
        true,
        true,
    )?;

    let request = Request {
        messages,
        model: index.get_model_by_name(&index.api_config.model)?,
        frequency_penalty: None,
        max_tokens: None,
        max_retry: index.api_config.max_retry,
        sleep_between_retries: index.api_config.sleep_between_retries,
        timeout: index.api_config.timeout,
        temperature: None,
        record_api_usage_at: index.api_config.dump_api_usage_at.clone().map(
            |path| RecordAt { path, id: String::from("extract_keywords") }
        ),
        dump_pdl_at: index.api_config.create_pdl_path("extract_keywords"),
        dump_json_at: index.api_config.dump_log_at.clone(),
        schema,
        schema_max_try: 3,
    };
    Ok(request.send_and_validate::<Keywords>(Keywords::default()).await?)
}
