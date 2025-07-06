use super::FileTree;
use crate::Keywords;
use crate::chunk::{Chunk, RenderableChunk};
use crate::error::Error;
use crate::index::Index;
use crate::query::QueryResponse;
use crate::uid::Uid;
use ragit_cli::substr_edit_distance;
use ragit_pdl::{Schema, escape_pdl_tokens};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
pub enum Action {
    ReadFile,
    ReadDir,
    SearchExact,
    SearchTfidf,

    /// This action will be filtered out if there's no summary.
    /// Please make sure to run `rag summary` if you want to
    /// use this action.
    GetSummary,

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
            Action::GetSummary,
            Action::SimpleRag,
        ]
    }

    // If this action requires an argument, the instruction must be "Give me an argument. The argument must be...".
    // If it doesn't require an argument, an AI will always reply "okay" to the instruction (I'll push a fake turn).
    pub(crate) fn get_instruction(&self) -> String {
        match self {
            Action::ReadFile => "Give me an exact path of a file that you want to read. Don't say anything other than the path of the file.",
            Action::ReadDir => "Give me an exact path of a directory that you want to read. Don't say anything other than the path of the directory.",
            Action::SearchExact => "Give me a keyword that you want to search for. It's not a pattern, just a keyword (case-sensitive). I'll use exact-text-matching to search. Don't say anything other than the keyword.",
            Action::SearchTfidf => "Give me a comma-separated list of keywords that you want to search for. Don't say anything other than the keywords.",
            Action::GetSummary => "I'll give you the summary. Hold on.",
            Action::SimpleRag => "Give me a simple factual question. Don't say anything other than the question.",
        }.to_string()
    }

    pub(crate) fn requires_argument(&self) -> bool {
        match self {
            Action::ReadFile => true,
            Action::ReadDir => true,
            Action::SearchExact => true,
            Action::SearchTfidf => true,
            Action::GetSummary => false,
            Action::SimpleRag => true,
        }
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
            Action::GetSummary => "Get summary of the entire knowledge-base.",
            Action::SimpleRag => "Call a simple RAG agent: if you ask a simple factual question, a RAG agent will read the files and answer your question. You can only ask a simple factual question, not complex reasoning questions.",
        }.to_string()
    }

    pub(crate) async fn run(&self, argument: &str, index: &Index) -> Result<ActionResult, Error> {
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

        let r = match self {
            Action::ReadFile => match index.processed_files.get(&argument) {
                Some(uid) => {
                    let chunk_uids = index.get_chunks_of_file(*uid)?;

                    // If the file is too long, it shows the summaries of its chunks
                    // instead of `cat-file`ing the file.
                    // TODO: what if it's sooooo long that even the chunk list is too long?
                    let max_chunks = index.query_config.max_retrieval;

                    // NOTE: Even an empty file has a chunk. So `.len()` must be greater than 0.
                    match chunk_uids.len() {
                        1 => {
                            let chunk = index.get_chunk_by_uid(chunk_uids[0])?.into_renderable(index, false  /* render_image */)?;
                            ActionResult::ReadFileShort {
                                chunk_uids,
                                rendered: chunk,
                            }
                        },
                        n if n <= max_chunks => {
                            let chunk_uids = index.get_chunks_of_file(*uid)?;
                            let chunk = index.get_merged_chunk_of_file(*uid)?;
                            ActionResult::ReadFileShort {
                                chunk_uids,
                                rendered: chunk,
                            }
                        },
                        _ => {
                            let mut chunks = Vec::with_capacity(chunk_uids.len());

                            for chunk_uid in chunk_uids.iter() {
                                chunks.push(index.get_chunk_by_uid(*chunk_uid)?);
                            }

                            ActionResult::ReadFileLong(chunks)
                        },
                    }
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
                    ActionResult::NoSuchFile {
                        file: argument.to_string(),
                        similar_files,
                    }
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
                    ActionResult::NoSuchDir {
                        dir: argument.to_string(),

                        // TODO: I want to suggest directories with a similar name,
                        //       but it's too tricky to find ones.
                        similar_dirs: vec![],
                    }
                }

                else {
                    ActionResult::ReadDir(file_tree)
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

                ActionResult::Search {
                    r#type: SearchType::from(*self),
                    keyword: argument.to_string(),
                    chunks,
                }
            },
            Action::GetSummary => {
                // The summary must exist. Otherwise, this action should have been filtered out.
                let summary = index.get_summary().unwrap();
                ActionResult::GetSummary(summary.to_string())
            },
            Action::SimpleRag => {
                let response = index.query(
                    &argument,
                    vec![],  // no history
                    None,  // no output schema
                ).await?;

                ActionResult::SimpleRag(response)
            },
        };

        Ok(r)
    }
}

#[derive(Clone, Debug, Serialize)]
pub enum ActionResult {
    // If the file is short enough, it'll merge its chunks into one.
    ReadFileShort {
        chunk_uids: Vec<Uid>,
        rendered: RenderableChunk,
    },
    ReadFileLong(Vec<Chunk>),
    NoSuchFile {
        file: String,
        similar_files: Vec<String>,
    },

    ReadDir(FileTree),
    NoSuchDir {
        dir: String,
        similar_dirs: Vec<String>,
    },
    Search {
        r#type: SearchType,
        keyword: String,
        chunks: Vec<Chunk>,
    },
    GetSummary(String),
    SimpleRag(QueryResponse),
}

impl ActionResult {
    // This is exactly what the AI sees (a turn).
    pub fn render(&self) -> String {
        match self {
            ActionResult::ReadFileShort { rendered, .. } => rendered.data.clone(),
            ActionResult::ReadFileLong(chunks) => format!(
                "The file is too long to show you. Instead, I'll show you the summaries of the chunks of the file.\n\n{}",
                chunks.iter().enumerate().map(
                    |(index, chunk)| format!(
                        "{}. {}\nsummary: {}",
                        index + 1,
                        escape_pdl_tokens(&chunk.render_source()),
                        escape_pdl_tokens(&chunk.summary),
                    )
                ).collect::<Vec<_>>().join("\n\n"),
            ),
            ActionResult::NoSuchFile { file, similar_files } => format!(
                "There's no such file: `{file}`{}",
                if !similar_files.is_empty() {
                    format!("\nThere are files with a similar name:\n\n{}", similar_files.join("\n"))
                } else {
                    String::new()
                },
            ),
            ActionResult::ReadDir(file_tree) => file_tree.render(),
            ActionResult::NoSuchDir { dir, similar_dirs } => format!(
                "There's no such dir: `{dir}`{}",
                if !similar_dirs.is_empty() {
                    format!("\nThere are dirs with a similar name:\n\n{}", similar_dirs.join("\n"))
                } else {
                    String::new()
                },
            ),
            ActionResult::Search { r#type, keyword, chunks } => {
                if chunks.is_empty() {
                    match r#type {
                        SearchType::Exact => format!("There's no file that contains the keyword `{keyword}`. Perhaps try tfidf search with the same keyword."),
                        SearchType::Tfidf => format!("There's no file that matches keywords `{keyword}`.")
                    }
                }

                else {
                    let header = format!(
                        "This is a list of chunks that {} `{keyword}`.",
                        match r#type {
                            SearchType::Exact => "contains the keyword",
                            SearchType::Tfidf => "matches keywords",
                        },
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
            ActionResult::GetSummary(summary) => summary.clone(),
            ActionResult::SimpleRag(response) => response.response.clone(),
        }
    }
}

// The primary goal of this struct is to render `agent.pdl`.
#[derive(Debug, Default, Serialize)]
pub struct ActionState {
    // A set of actions that it can run.
    #[serde(skip)]
    pub actions: Vec<Action>,

    // It uses an index of an action instead of the action itself.
    // That's because it's tricky to (de)serialize actions.
    pub index: Option<usize>,
    pub instruction: Option<String>,
    pub argument: Option<String>,
    pub result: Option<ActionResult>,

    // What the AI sees
    pub result_rendered: Option<String>,

    // If yes, it runs another action within the same context
    pub r#continue: Option<String>,  // "yes" | "no"
}

impl ActionState {
    pub fn new(actions: Vec<Action>) -> Self {
        ActionState {
            actions,
            ..ActionState::default()
        }
    }

    pub fn get_schema(&self) -> Option<Schema> {
        if self.index.is_none() {
            Some(Schema::integer_between(Some(1), Some(self.actions.len() as i128)))
        }

        else if self.argument.is_none() {
            None
        }

        else if self.r#continue.is_none() {
            Some(Schema::default_yesno())
        }

        else {
            unreachable!()
        }
    }

    pub async fn update(
        &mut self,
        input: Value,
        index: &Index,
        action_traces: &mut Vec<ActionTrace>,
    ) -> Result<(), Error> {
        if self.index.is_none() {
            // If `input.as_u64()` fails, that means the AI is so stupid
            // that it cannot choose a number even with pdl schema's help.
            // So we just choose an arbitrary action. The AI's gonna fail
            // anyway and will break soon.
            let n = input.as_u64().unwrap_or(1) as usize;
            let action = self.actions[n - 1];  // AI uses 1-based index
            self.index = Some(n);
            self.instruction = Some(action.get_instruction());

            if !action.requires_argument() {
                // See comments in `Action::get_instruction`
                self.argument = Some(String::from("okay"));
                self.result = Some(action.run("", index).await?);
                self.result_rendered = self.result.clone().map(|r| r.render());
                action_traces.push(ActionTrace {
                    action,
                    argument: None,
                    result: self.result.clone().unwrap(),
                });
            }
        }

        else if self.argument.is_none() {
            // NOTE: pdl schema `string` is infallible
            let input = input.as_str().unwrap();
            let action = self.actions[self.index.unwrap() - 1];  // AI uses 1-based index
            self.argument = Some(input.to_string());
            self.result = Some(action.run(input, index).await?);
            self.result_rendered = self.result.clone().map(|r| r.render());
            action_traces.push(ActionTrace {
                action,
                argument: Some(input.to_string()),
                result: self.result.clone().unwrap(),
            });
        }

        else if self.r#continue.is_none() {
            // If `input.as_bool()` fails, that means the AI is
            // not smart enough to generate a boolean. There's
            // no need to continue.
            let input = input.as_bool().unwrap_or(false);
            let s = if input { "yes" } else { "no" };

            self.r#continue = Some(s.to_string());
        }

        else {
            unreachable!()
        }

        Ok(())
    }
}

#[derive(Serialize)]
pub struct ActionTrace {
    pub action: Action,
    pub argument: Option<String>,
    pub result: ActionResult,
}

#[derive(Clone, Debug, Serialize)]
enum SearchType {
    Exact,
    Tfidf,
}

impl From<Action> for SearchType {
    fn from(a: Action) -> SearchType {
        match a {
            Action::SearchExact => SearchType::Exact,
            Action::SearchTfidf => SearchType::Tfidf,
            _ => panic!(),
        }
    }
}
