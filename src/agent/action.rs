use super::FileTree;
use crate::Keywords;
use crate::error::Error;
use crate::index::Index;
use ragit_cli::substr_edit_distance;
use ragit_pdl::escape_pdl_tokens;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Action {
    ReadFile,
    ReadDir,
    SearchExact,
    SearchTfidf,

    /// I'm using the term "simple" because I want to make sure that it's not an agentic RAG.
    SimpleRag,
}

impl Action {
    pub fn all_actions() -> Vec<Action> {
        vec![
            Action::ReadFile,
            Action::ReadDir,
            Action::SearchExact,
            Action::SearchTfidf,
            Action::SimpleRag,
        ]
    }

    pub(crate) fn get_instruction(&self) -> String {
        match self {
            Action::ReadFile => "Give me an exact path of a file that you want to read. Don't say anything other than the path of the file.",
            Action::ReadDir => "Give me an exact path of a directory that you want to read. Don't say anything other than the path of the directory.",
            Action::SearchExact => "Give me a keyword that you want to search for. It's not a pattern, just a keyword (case-sensitive). I'll use exact-text-matching to search. Don't say anything other than the keyword.",
            Action::SearchTfidf => "Give me a comma-separated list of keywords that you want to search for. Don't say anything other than the keywords.",
            Action::SimpleRag => "Give me a simple factual question. Don't say anything other than the question.",
        }.to_string()
    }

    pub(crate) fn write_prompt(actions: &[Action]) -> String {
        actions.iter().enumerate().map(
            |(i, p)| format!("{}. {}", i + 1, p.write_unit_prompt())
        ).collect::<Vec<_>>().join("\n")
    }

    pub(crate) fn write_unit_prompt(&self) -> String {
        match self {
            Action::ReadFile => "Read a file: if you give me the exact path of a file, I'll show you the content of the file.",
            Action::ReadDir => "See a list of files in a directory: if you give me the exact path of a directory, I'll show you a list of the files in the directory.",
            Action::SearchExact => "Search by a keyword (exact): if you give me a keyword, I'll give you a list of files that contain the exact keyword in their contents.",
            Action::SearchTfidf => "Search by keywords (tfidf): if you give me keywords, I'll give you a tfidf search result. It tries to search for files that contain any of the keywords, even though there's no exact match.",
            Action::SimpleRag => "Call a simple RAG agent: if you ask a simple factual question, a RAG agent will read the files and answer your question. You can only ask a simple factual question, not complex reasoning questions.",
        }.to_string()
    }

    pub(crate) async fn run(&self, argument: &str, index: &Index) -> Result<String, Error> {
        let mut argument = argument.trim().to_string();

        // argument is a path
        if let Action::ReadFile | Action::ReadDir = self {
            // If `normalize` fails, that means `argument` is not a valid path,
            // and it will throw an error later.
            argument = ragit_fs::normalize(&argument).unwrap_or(argument);

            if argument.starts_with("/") {
                argument = argument.get(1..).unwrap().to_string();
            }
        }

        let s = match self {
            Action::ReadFile => match index.processed_files.get(&argument) {
                Some(uid) => {
                    let chunk = index.get_merged_chunk_of_file(*uid)?;
                    chunk.data
                },
                None => {
                    let mut similar_files = vec![];

                    // TODO: it might take very very long time if the knowledge-base is large...
                    for file in index.processed_files.keys() {
                        let dist = substr_edit_distance(argument.as_bytes(), file.as_bytes());

                        if dist < 3 {
                            similar_files.push((file.to_string(), dist));
                        }
                    }

                    similar_files.sort_by_key(|(_, d)| *d);

                    if similar_files.len() > 10 {
                        similar_files = similar_files[..10].to_vec();
                    }

                    let similar_files = similar_files.into_iter().map(|(f, _)| f).collect::<Vec<_>>();
                    format!(
                        "There's no such file: `{argument}`{}",
                        if !similar_files.is_empty() {
                            format!("\nThere are files with a similar name:\n\n{}", similar_files.join("\n"))
                        } else {
                            String::new()
                        },
                    )
                },
            },
            Action::ReadDir => {
                if !argument.ends_with("/") {
                    argument = format!("{argument}/");
                }

                let mut file_tree = FileTree::root();

                for file in index.processed_files.keys() {
                    if file.starts_with(&argument) {
                        file_tree.insert(file.get(argument.len()..).unwrap());
                    }
                }

                if file_tree.is_empty() {
                    // TODO: I want to suggest directories with a similar name,
                    //       but it's too tricky to find ones.
                    format!("There's no such directory: `{argument}`")
                }

                else {
                    file_tree.render()
                }
            },
            Action::SearchExact | Action::SearchTfidf => {
                // The result of exact search is a subset of the result of tfidf search.
                let mut limit = if *self == Action::SearchExact {
                    100
                } else {
                    10
                };

                let chunks = 'chunks_loop: loop {
                    let candidates = index.run_tfidf(
                        Keywords::from_raw(vec![argument.to_string()]),
                        limit,
                    )?;
                    let mut chunks = Vec::with_capacity(candidates.len());
                    let mut chunks_exact_match = vec![];

                    for c in candidates.iter() {
                        chunks.push(index.get_chunk_by_uid(c.id)?);
                    }

                    if *self == Action::SearchTfidf {
                        break chunks;
                    }

                    for chunk in chunks.iter() {
                        if chunk.data.contains(&argument) {
                            chunks_exact_match.push(chunk.clone());

                            if chunks_exact_match.len() == 10 {
                                break 'chunks_loop chunks_exact_match;
                            }
                        }
                    }

                    // We have a complete set of the tfidf result, so there's
                    // no point in increasing the limit.
                    if candidates.len() < limit || limit == index.chunk_count {
                        break chunks_exact_match;
                    }

                    // Maybe we can get more exact-matches if we increase the
                    // limit of the tfidf-match.
                    limit = (limit * 5).min(index.chunk_count);
                };

                if chunks.is_empty() {
                    if *self == Action::SearchExact {
                        format!("There's no file that contains the keyword `{argument}`. Perhaps try tfidf search with the same keyword.")
                    }

                    else {
                        format!("There's no file that matches keywords `{argument}`.")
                    }
                }

                else {
                    let header = format!(
                        "This is a list of chunks that {} `{argument}`.",
                        if *self == Action::SearchExact { "contains the keyword" } else { "matches keywords" },
                    );

                    format!(
                        "{header}\n\n{}",
                        chunks.iter().enumerate().map(
                            |(index, chunk)| format!(
                                "{}. {}\nsummary: {}",
                                index + 1,
                                escape_pdl_tokens(&chunk.render_source()),
                                escape_pdl_tokens(&chunk.summary),
                            )
                        ).collect::<Vec<_>>().join("\n\n")
                    )
                }
            },
            Action::SimpleRag => {
                let response = index.query(
                    &argument,
                    vec![],  // no history
                    None,  // no output schema
                ).await?;

                response.response
            },
        };

        Ok(s)
    }
}
