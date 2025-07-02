// Playground for `summarize_knowledge_base.pdl`
// This is an agent that generates a summary of the entire knowledge-base. If this agent works well, I'll develop a general-purpose agent for ragit.

use crate::Keywords;
use crate::error::Error;
use crate::index::Index;
use ragit_api::Request;
use ragit_cli::substr_edit_distance;
use ragit_fs::extension;
use ragit_pdl::{
    Pdl,
    Schema,
    escape_pdl_tokens,
    into_context,
    parse_pdl,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize)]
struct SummaryAgentState {
    question: String,
    single_paragraph: bool,
    context: String,
    needed_information: Option<String>,
    actions: Vec<Action>,
    is_actions_complete: bool,
    new_information: Option<String>,
    new_context: Option<String>,

    // when it `has_enough_information` it writes the
    // final result to `result` field and break
    has_enough_information: bool,
    result: Option<String>,
}

impl SummaryAgentState {
    pub fn get_schema(&self) -> Option<Schema> {
        if self.has_enough_information {
            None
        }

        else if self.needed_information.is_none() {
            None
        }

        else if !self.check_actions_complete() {
            self.actions.last().unwrap().get_schema()
        }

        else {
            None
        }
    }

    pub async fn update(&mut self, input: Value, index: &Index) -> Result<(), Error> {
        if self.has_enough_information {
            self.result = Some(input.as_str().unwrap().to_string());
        }

        else if self.needed_information.is_none() {
            self.needed_information = Some(input.as_str().unwrap().to_string());
            self.actions.push(Action::default());
        }

        else if !self.check_actions_complete() {
            let last_action = self.actions.last_mut().unwrap();
            last_action.update(input, index).await?;

            match last_action.r#continue.as_ref().map(|s| s.as_str()) {
                Some("yes") => {
                    self.actions.push(Action::default());
                },
                Some("no") => {
                    self.is_actions_complete = true;
                },
                Some(s) => panic!("something went wrong: {s:?}"),
                _ => {},
            }
        }

        else if self.new_information.is_none() {
            self.new_information = Some(input.as_str().unwrap().to_string());
        }

        else if self.new_context.is_none() {
            self.new_context = Some(input.as_str().unwrap().to_string());
        }

        else {
            unreachable!()
        }

        Ok(())
    }

    fn check_actions_complete(&self) -> bool {
        if let Some(action) = self.actions.last() {
            action.r#continue.as_ref().map(|s| s.as_str()) == Some("no")
        }

        else {
            false
        }
    }
}

impl Default for SummaryAgentState {
    fn default() -> Self {
        SummaryAgentState {
            question: String::new(),
            single_paragraph: false,
            context: String::new(),
            needed_information: None,
            actions: vec![],
            is_actions_complete: false,
            new_information: None,
            new_context: None,
            has_enough_information: false,
            result: None,
        }
    }
}

#[derive(Debug, Default, Serialize)]
struct Action {
    // 1. Read a file
    // 2. See a list of files in a directory
    // 3. Search by a keyword (exact)
    // 4. Search by keywords (tfidf)
    // 5. Call a simple RAG agent
    num: Option<u32>,
    instruction: Option<String>,
    argument: Option<String>,
    result: Option<String>,
    r#continue: Option<String>,  // "yes" | "no"
}

impl Action {
    pub fn get_schema(&self) -> Option<Schema> {
        if self.num.is_none() {
            Some(Schema::integer_between(Some(1), Some(5)))
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

    pub async fn update(&mut self, input: Value, index: &Index) -> Result<(), Error> {
        if self.num.is_none() {
            let n = input.as_u64().unwrap() as u32;
            self.num = Some(n);
            self.instruction = Some(
                match n {
                    // Read a file
                    1 => "Give me an exact path of a file that you want to read. Don't say anything other than the path of the file.",
                    // See a list of files in a directory
                    2 => "Give me an exact path of a directory that you want to read. Don't say anything other than the path of the directory.",
                    // Search by a keyword (exact)
                    3 => "Give me a keyword that you want to search for. It's not a pattern, just a keyword (case-sensitive). I'll use exact-text-matching to search. Don't say anything other than the keyword.",
                    // Search by keywords (tfidf)
                    4 => "Give me a comma-separated list of keywords that you want to search for. Don't say anything other than the keywords.",
                    // Call a simple RAG agent
                    5 => "Give me a simple factual question. Don't say anything other than the question.",
                    _ => unreachable!(),
                }.to_string()
            );
        }

        else if self.argument.is_none() {
            let input = input.as_str().unwrap();
            let mut argument = input.trim().to_string();

            // argument is a path
            if self.num.unwrap() == 1 || self.num.unwrap() == 2 {
                // If `normalize` fails, that means `argument` is not a valid path,
                // and it will throw an error later.
                argument = ragit_fs::normalize(&argument).unwrap_or(argument);

                if argument.starts_with("/") {
                    argument = argument.get(1..).unwrap().to_string();
                }
            }

            self.argument = Some(argument.to_string());
            self.result = Some(
                match self.num.unwrap() {
                    // Read a file
                    1 => match index.processed_files.get(&argument) {
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
                    // See a list of files in a directory
                    2 => {
                        if !argument.ends_with("/") {
                            argument = format!("{argument}/");
                        }

                        let mut file_tree = FileTree::root();

                        for file in index.processed_files.keys() {
                            if file.starts_with(&argument) {
                                let path_elements = file.get(argument.len()..).unwrap().split(|s| s == '/').filter(|s| !s.is_empty()).collect::<Vec<_>>();
                                add_path_to_tree(&mut file_tree, &path_elements);
                            }
                        }

                        if file_tree.is_empty() {
                            format!("There's no such directory: `{argument}`")
                        }

                        else {
                            file_tree.render()
                        }
                    },
                    // 3. Search by a keyword (exact)
                    // 4. Search by keywords (tfidf)
                    n @ (3 | 4) => {
                        // The result of exact search is a subset of the result of tfidf search.
                        let mut limit = if n == 3 {
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

                            if n == 4 {
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
                            if n == 3 {
                                format!("There's no file that contains the keyword `{argument}`. Perhaps try tfidf search with the same keyword.")
                            }

                            else {
                                format!("There's no file that matches keywords `{argument}`.")
                            }
                        }

                        else {
                            let header = format!(
                                "This is a list of chunks that {} `{argument}`.",
                                if n == 3 { "contains the keyword" } else { "matches keywords" },
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
                    // Call a simple RAG agent
                    5 => {
                        let response = index.query(
                            &argument,
                            vec![],  // no history
                            None,  // no output schema
                        ).await?;

                        response.response
                    },
                    _ => unreachable!(),
                }
            );
        }

        else if self.r#continue.is_none() {
            let input = input.as_bool().unwrap();
            let s = if input { "yes" } else { "no" };

            self.r#continue = Some(s.to_string());
        }

        else {
            unreachable!()
        }

        Ok(())
    }
}

impl Index {
    pub async fn summarize(&self) -> Result<String, Error> {
        let mut state = SummaryAgentState::default();
        state.single_paragraph = true;
        state.question = String::from("Give me a summary of the knowledge-base.");
        state.context = self.get_rough_summary()?;
        let mut context_update = 0;

        loop {
            state = self.summarize_step(state).await?;

            if state.has_enough_information {
                return Ok(state.result.unwrap());
            }

            if let Some(context) = &state.new_context {
                let context = context.to_string();
                state = SummaryAgentState::default();
                state.context = context;
                context_update += 1;

                // TODO: I want LLMs to decide whether it has enough information.
                //       But it often falls into an infinite loop.
                if context_update == 2 {
                    state.has_enough_information = true;
                }
            }
        }
    }

    async fn summarize_step(&self, mut state: SummaryAgentState) -> Result<SummaryAgentState, Error> {
        let schema = state.get_schema();
        let Pdl { messages, .. } = parse_pdl(
            &self.get_prompt("summarize_knowledge_base")?,
            &into_context(&state)?,
            "/",  // TODO: `<|media|>` is not supported for this prompt
            true,
            true,
        )?;
        let request = Request {
            messages,
            model: self.get_model_by_name(&self.api_config.model)?,
            max_retry: self.api_config.max_retry,
            sleep_between_retries: self.api_config.sleep_between_retries,
            timeout: self.api_config.timeout,
            dump_api_usage_at: self.api_config.dump_api_usage_at(&self.root_dir, "summarize_knowledge_base"),
            dump_pdl_at: self.api_config.create_pdl_path(&self.root_dir, "summarize_knowledge_base"),
            dump_json_at: self.api_config.dump_log_at(&self.root_dir),
            schema: schema.clone(),
            schema_max_try: 3,
            ..Request::default()
        };
        let response = if schema.is_some() {
            request.send_and_validate::<Value>(Value::Null).await?
        } else {
            let r = request.send().await?;
            Value::String(r.get_message(0).unwrap().to_string())
        };

        state.update(response, self).await?;
        Ok(state)
    }

    fn get_rough_summary(&self) -> Result<String, Error> {
        // HashMap<extension, number of chunks>
        // It counts chunks instead of files because files have variable lengths,
        // but chunks have a length limit.
        let mut count_by_extension = HashMap::new();

        // It's not always equal to `self.chunk_count` because some chunks are not
        // from a file.
        let mut file_chunk_count = 0;

        let mut file_tree = FileTree::root();
        let mut char_len = 0;
        let mut image_uids = HashSet::new();

        for (file, uid) in self.processed_files.iter() {
            let chunks = self.get_chunks_of_file(*uid)?;
            let extension = extension(file)?
                .map(|e| format!(".{e}"))
                .unwrap_or_else(|| String::from("no extension"));
            let path_elements = file.split(|s| s == '/').filter(|s| !s.is_empty()).collect::<Vec<_>>();
            add_path_to_tree(&mut file_tree, &path_elements);
            file_chunk_count += chunks.len();

            match count_by_extension.get_mut(&extension) {
                Some(n) => { *n += chunks.len() },
                _ => { count_by_extension.insert(extension, chunks.len()); },
            }

            for chunk in chunks.iter() {
                let chunk = self.get_chunk_by_uid(*chunk)?;
                char_len += chunk.char_len;

                for image in chunk.images.iter() {
                    image_uids.insert(*image);
                }
            }
        }

        let mut count_by_extension = count_by_extension.into_iter().collect::<Vec<_>>();
        count_by_extension.sort_by_key(|(_, count)| *count);

        let common_extensions = match count_by_extension.len() {
            0 => String::from(""),
            1 => format!("All the files have the same extension: `{}`", count_by_extension[0].0),
            2 => format!(
                "The files' extension is either `{}` ({:.3} %) or `{}` ({:.3} %)",
                count_by_extension[0].0, count_by_extension[0].1 as f64 * 100.0 / file_chunk_count as f64,
                count_by_extension[1].0, count_by_extension[1].1 as f64 * 100.0 / file_chunk_count as f64,
            ),
            _ => format!(
                "The most common extensions of the files are `{}` ({:.3} %), `{}` ({:.3} %) and `{}` ({:.3} %)",
                count_by_extension[0].0, count_by_extension[0].1 as f64 * 100.0 / file_chunk_count as f64,
                count_by_extension[1].0, count_by_extension[1].1 as f64 * 100.0 / file_chunk_count as f64,
                count_by_extension[2].0, count_by_extension[2].1 as f64 * 100.0 / file_chunk_count as f64,
            ),
        };

        Ok(format!(
            "This knowledge-base consists of {} files ({} characters of text and {} images). {}\nBelow is the list of the files and directories in the knowledge-base.\n\n{}",
            self.processed_files.len(),
            char_len,
            image_uids.len(),
            common_extensions,
            file_tree.render(),
        ))
    }
}

#[derive(Debug, Serialize)]
struct FileTree {
    is_dir: bool,
    children: HashMap<String, FileTree>,
}

impl FileTree {
    pub fn root() -> Self {
        FileTree {
            is_dir: true,
            children: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        if self.is_dir {
            self.children.values().map(|f| f.len()).sum()
        }

        else {
            1
        }
    }

    pub fn is_empty(&self) -> bool {
        self.is_dir && self.children.is_empty()
    }

    pub fn to_paths(&self) -> Vec<String> {
        let mut result = vec![];

        for (k, v) in self.children.iter() {
            if v.is_dir {
                for p in v.to_paths().iter() {
                    result.push(format!("{k}/{p}"));
                }
            }

            else {
                result.push(k.to_string());
            }
        }

        result
    }

    // NOTE: There are a lot of hard-coded integers in this function. They're all
    //       just arbitrary numbers. I haven't done enough tests on them, and I
    //       have to.
    //       I don't want to make these configurable; that'd confuse
    //       new-comers, right?
    pub fn render(&self) -> String {
        let total_files: usize = self.children.values().map(|f| f.len()).sum();

        if total_files < 30 {
            let mut paths = self.to_paths();
            paths.sort();
            paths.join("\n")
        }

        else {
            let mut dirs = vec![];
            let mut excessive_dirs = 0;
            let mut files = vec![];
            let mut excessive_files = 0;

            for (k, v) in self.children.iter() {
                if v.is_dir {
                    dirs.push((k.to_string(), v.len()));
                }

                else {
                    files.push(k.to_string());
                }
            }

            dirs.sort_by_key(|(d, _)| d.to_string());
            files.sort();

            if dirs.len() > 15 {
                excessive_dirs = dirs.len() - 15;
                dirs = dirs[..15].to_vec();
            }

            if files.len() > 15 {
                excessive_files = files.len() - 15;
                files = files[..15].to_vec();
            }

            let mut lines = vec![
                dirs.iter().map(
                    |(d, f)| format!("{d}/    ({f} files in it)")
                ).collect::<Vec<_>>(),
                files,
            ].concat();

            match (excessive_dirs, excessive_files) {
                (0, 0) => {},
                (0, f) => {
                    lines.push(format!("... and {f} more files"));
                },
                (d, 0) => {
                    lines.push(format!("... and {d} more directories"));
                },
                (d, f) => {
                    lines.push(format!("... and {d} more directories and {f} more files"));
                },
            }

            lines.join("\n")
        }
    }
}

fn add_path_to_tree(tree: &mut FileTree, path_elements: &[&str]) {
    if path_elements.len() == 1 {
        tree.children.insert(
            path_elements[0].to_string(),
            FileTree {
                is_dir: false,
                children: HashMap::new(),
            },
        );
    }

    else {
        let dir_name = &path_elements[0];

        match tree.children.get_mut(*dir_name) {
            Some(f) => {
                add_path_to_tree(f, &path_elements[1..]);
            },
            None => {
                let mut children = FileTree {
                    is_dir: true,
                    children: HashMap::new(),
                };
                add_path_to_tree(&mut children, &path_elements[1..]);
                tree.children.insert(
                    dir_name.to_string(),
                    children,
                );
            },
        }
    }
}
