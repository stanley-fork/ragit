use super::QueryTurn;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RenderedQueryTurn {
    pub role: String,
    pub content: String,
}

pub fn render_query_turns(turns: &[QueryTurn]) -> Vec<RenderedQueryTurn> {
    let mut result = Vec::with_capacity(turns.len() * 2);

    for turn in turns.iter() {
        result.push(RenderedQueryTurn {
            role: String::from("user"),
            content: turn.query.clone(),
        });
        result.push(RenderedQueryTurn {
            // how about using the model name?
            role: String::from("assistant"),
            content: format!(
                "{}{}",
                turn.response.response,
                if turn.response.retrieved_chunks.is_empty() {
                    String::new()
                } else {
                    format!(
                        "\n---- source{} ----\n{}",
                        if turn.response.retrieved_chunks.len() == 1 { "" } else { "s" },
                        turn.response.retrieved_chunks.iter().map(
                            |chunk| format!("{} ({})", chunk.render_source(), chunk.uid.abbrev(8))
                        ).collect::<Vec<_>>().join("\n"),
                    )
                },
            ),
        });
    }

    result
}
