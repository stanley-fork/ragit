use chrono::Local;
use crate::error::Error;
use crate::index::Index;
use ragit_api::Request;
use ragit_fs::{
    WriteMode,
    extension,
    join,
    write_string,
};
use ragit_pdl::{
    Pdl,
    Schema,
    into_context,
    parse_pdl,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

mod action;
mod file_tree;

pub use action::Action;
pub use file_tree::FileTree;

#[derive(Debug, Serialize)]
struct AgentState {
    question: String,
    single_paragraph: bool,
    context: String,
    needed_information: Option<String>,
    actions: Vec<ActionTrace>,
    is_actions_complete: bool,
    new_information: Option<String>,
    new_context: Option<String>,

    // when it `has_enough_information` it writes the
    // final result to `result` field and break
    has_enough_information: bool,
    result: Option<String>,
}

impl AgentState {
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
            self.actions.push(ActionTrace::default());
        }

        else if !self.check_actions_complete() {
            let last_action = self.actions.last_mut().unwrap();
            last_action.update(input, index).await?;

            match last_action.r#continue.as_ref().map(|s| s.as_str()) {
                Some("yes") => {
                    self.actions.push(ActionTrace::default());
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

impl Default for AgentState {
    fn default() -> Self {
        AgentState {
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
struct ActionTrace {
    // It uses an index of an action instead of the action itself.
    // That's because it's tricky to (de)serialize actions.
    index: Option<usize>,
    instruction: Option<String>,
    argument: Option<String>,
    result: Option<String>,
    r#continue: Option<String>,  // "yes" | "no"
}

impl ActionTrace {
    pub fn get_schema(&self) -> Option<Schema> {
        if self.index.is_none() {
            Some(Schema::integer_between(Some(1), Some(Action::MAX_INDEX as i128)))
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
        if self.index.is_none() {
            let n = input.as_u64().unwrap() as usize;
            let action = Action::from_index(n)?;
            self.index = Some(n);
            self.instruction = Some(action.get_instruction());
        }

        else if self.argument.is_none() {
            let input = input.as_str().unwrap();
            let action = Action::from_index(self.index.unwrap())?;
            self.argument = Some(input.to_string());
            self.result = Some(action.run(input, index).await?);
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
    pub async fn agent(
        &self,
        question: &str,
        single_paragraph: bool,
        initial_context: String,
    ) -> Result<String, Error> {
        let mut state = AgentState::default();
        state.single_paragraph = single_paragraph;
        state.question = question.to_string();
        state.context = initial_context;
        let mut context_update = 0;

        loop {
            state = self.step_agent(state).await?;

            if state.has_enough_information {
                return Ok(state.result.unwrap());
            }

            if let Some(context) = &state.new_context {
                let context = context.to_string();
                state = AgentState::default();
                state.single_paragraph = single_paragraph;
                state.question = question.to_string();
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

    async fn step_agent(&self, mut state: AgentState) -> Result<AgentState, Error> {
        let schema = state.get_schema();
        let Pdl { messages, .. } = parse_pdl(
            &self.get_prompt("agent")?,
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
            dump_api_usage_at: self.api_config.dump_api_usage_at(&self.root_dir, "agent"),
            dump_pdl_at: self.api_config.create_pdl_path(&self.root_dir, "agent"),
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

        if let Some(log_at) = self.api_config.dump_log_at(&self.root_dir) {
            let now = Local::now();
            write_string(
                &join(
                    &log_at,
                    &format!("agent-state-{}.json", now.to_rfc3339()),
                )?,
                &serde_json::to_string_pretty(&state)?,
                WriteMode::CreateOrTruncate,
            )?;
        }

        Ok(state)
    }

    /// This is an initial context of a summary agent.
    pub fn get_rough_summary(&self) -> Result<String, Error> {
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
            file_tree.insert(file);
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
