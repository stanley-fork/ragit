use lazy_static::lazy_static;
use std::collections::HashMap;

pub const PROMPT_DIR: &str = "prompts";

lazy_static! {
    pub static ref PROMPTS: HashMap<String, String> = {
        let mut result = HashMap::new();
        result.insert(
            String::from("answer_query"),
            include_str!(".././prompts/answer_query.pdl").to_string(),
        );
        result.insert(
            String::from("describe_image"),
            include_str!(".././prompts/describe_image.pdl").to_string(),
        );
        result.insert(
            String::from("extract_keyword"),
            include_str!(".././prompts/extract_keyword.pdl").to_string(),
        );
        result.insert(
            String::from("multi_turn"),
            include_str!(".././prompts/multi_turn.pdl").to_string(),
        );
        result.insert(
            String::from("raw"),
            include_str!(".././prompts/raw.pdl").to_string(),
        );
        result.insert(
            String::from("rerank_summary"),
            include_str!(".././prompts/rerank_summary.pdl").to_string(),
        );
        result.insert(
            String::from("rerank_title"),
            include_str!(".././prompts/rerank_title.pdl").to_string(),
        );
        result.insert(
            String::from("summarize"),
            include_str!(".././prompts/summarize.pdl").to_string(),
        );

        result
    };
}
