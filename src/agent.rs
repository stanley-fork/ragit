use chrono::Local;
use crate::chunk::Chunk;
use crate::error::Error;
use crate::index::Index;
use ragit_api::Request;
use ragit_fs::{
    WriteMode,
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

pub use action::{Action, ActionResult};
use action::{ActionState, ActionTrace};
pub use file_tree::FileTree;

// `derive(Serialize) for AgentState` has 2 purposes.
//
// 1. Dump log in `.ragit/logs`.
// 2. Create `tera::Context` for `agent.pdl`. The context is fed to the AI.
//
// I'm NOT gonna `derive(Deserialize)`. There are too many edge cases in
// deserializing the state.
#[derive(Debug, Serialize)]
struct AgentState {
    question: String,
    context: String,
    needed_information: Option<String>,
    action_states: Vec<ActionState>,

    // Actions that this agent can run.
    // It's necessary because agents have different sets of capabilities.
    // For example, the summary agent cannot run `Action::GetSummary` because
    // there's no summary yet!
    #[serde(skip)]
    actions: Vec<Action>,

    // The final response must be in this schema.
    #[serde(skip)]
    response_schema: Option<Schema>,

    // A summary of the knowledge-base
    // If there's no up-to-date summary, it doesn't create a new one. You have
    // to run `rag summary` to create a new one.
    summary: Option<String>,

    // It's generated from `actions`.
    // It's fed to the AI's context.
    action_prompt: String,

    // I want to use the term `has_action_to_run`, but serde_json doesn't allow that :(
    is_actions_complete: bool,
    new_information: Option<String>,
    new_context: Option<String>,

    // when it `has_enough_information` it writes the
    // final result to `response` field and break
    has_enough_information: bool,
    response: Option<String>,
}

impl AgentState {
    pub fn get_schema(&self) -> Option<Schema> {
        if self.has_enough_information {
            self.response_schema.clone()
        }

        else if self.needed_information.is_none() {
            None
        }

        else if self.has_action_to_run() {
            self.action_states.last().unwrap().get_schema()
        }

        else {
            None
        }
    }

    pub async fn update(
        &mut self,
        input: Value,
        index: &Index,
        action_traces: &mut Vec<ActionTrace>,
    ) -> Result<(), Error> {
        if self.has_enough_information {
            self.response = Some(input.as_str().unwrap().to_string());
        }

        else if self.needed_information.is_none() {
            self.needed_information = Some(input.as_str().unwrap().to_string());
            self.action_states.push(ActionState::new(self.actions.clone()));
        }

        else if self.has_action_to_run() {
            let last_action = self.action_states.last_mut().unwrap();
            last_action.update(input, index, action_traces).await?;

            match last_action.r#continue.as_ref().map(|s| s.as_str()) {
                Some("yes") => {
                    self.action_states.push(ActionState::new(self.actions.clone()));
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

    pub fn update_context(&mut self, context: String) {
        self.context = context;
        self.needed_information = None;
        self.action_states = vec![];
        self.is_actions_complete = false;
        self.new_information = None;
        self.new_context = None;
        self.has_enough_information = false;
        self.response = None;
    }

    fn has_action_to_run(&self) -> bool {
        if let Some(action) = self.action_states.last() {
            action.r#continue.as_ref().map(|s| s.as_str()) != Some("no")
        }

        else {
            true
        }
    }
}

impl Default for AgentState {
    fn default() -> Self {
        AgentState {
            question: String::new(),
            context: String::new(),
            needed_information: None,
            actions: vec![],
            response_schema: None,
            summary: None,
            action_prompt: String::new(),
            action_states: vec![],
            is_actions_complete: false,
            new_information: None,
            new_context: None,
            has_enough_information: false,
            response: None,
        }
    }
}

#[derive(Serialize)]
pub struct AgentResponse {
    pub response: String,
    pub actions: Vec<ActionTrace>,
}

impl AgentResponse {
    pub fn retrieved_chunks(&self, index: &Index) -> Result<Vec<Chunk>, Error> {
        let mut chunks = HashMap::new();

        for action in self.actions.iter() {
            match &action.result {
                ActionResult::ReadFileShort { chunk_uids, .. } => {
                    for chunk_uid in chunk_uids.iter() {
                        chunks.insert(*chunk_uid, index.get_chunk_by_uid(*chunk_uid)?);
                    }
                },
                ActionResult::SimpleRag(rag) => {
                    for chunk in rag.retrieved_chunks.iter() {
                        chunks.insert(chunk.uid, chunk.clone());
                    }
                },
                ActionResult::ReadChunk(chunk) => {
                    chunks.insert(chunk.uid, chunk.clone());
                },

                // `ReadFileLong` and `Search` have chunks, but many of them are
                // irrelevant. The AI will likely to choose relevant ones from them
                // and call `Action::ReadChunk`, which will be counted.
                ActionResult::ReadFileLong(_)
                | ActionResult::NoSuchFile { .. }
                | ActionResult::ReadDir(_)
                | ActionResult::NoSuchDir { .. }
                | ActionResult::NoSuchChunk(_)
                | ActionResult::ReadChunkAmbiguous { .. }
                | ActionResult::ReadChunkTooMany { .. }
                | ActionResult::Search { .. }
                | ActionResult::GetMeta { .. }
                | ActionResult::NoSuchMeta { .. } => {},
            }
        }

        Ok(chunks.into_values().collect())
    }
}

impl Index {
    pub async fn agent(
        &self,
        question: &str,
        initial_context: String,

        // list of available actions
        mut actions: Vec<Action>,

        // You can set the schema of the response.
        schema: Option<Schema>,

        // When running `rag summary`, we have to hide the summary.
        // Otherwise, the AI would just copy-paste the summary.
        hide_summary: bool,
    ) -> Result<AgentResponse, Error> {
        // dedup
        actions = actions.into_iter().collect::<HashSet<_>>().into_iter().collect();

        // It cannot get metadata if there's no metadata.
        if self.get_all_meta().unwrap_or_else(|_| HashMap::new()).is_empty() {
            actions = actions.into_iter().filter(
                |action| *action != Action::GetMeta
            ).collect();
        }

        let mut state = AgentState::default();
        state.question = question.to_string();
        state.context = initial_context;
        state.actions = actions.clone();
        state.action_prompt = Action::write_prompt(&actions);
        state.response_schema = schema.clone();

        if !hide_summary {
            state.summary = self.get_summary().map(|s| s.to_string());
        }

        let mut context_update = 0;
        let mut action_traces = vec![];

        loop {
            state = self.step_agent(state, &mut action_traces).await?;

            if state.has_enough_information {
                return Ok(AgentResponse {
                    response: state.response.unwrap(),
                    actions: action_traces,
                });
            }

            if let Some(context) = &state.new_context {
                let context = context.clone();
                state.update_context(context);
                context_update += 1;

                // TODO: I want LLMs to decide whether it has enough information.
                //       But it often falls into an infinite loop.
                if context_update == 2 {
                    state.has_enough_information = true;
                }
            }
        }
    }

    async fn step_agent(&self, mut state: AgentState, action_traces: &mut Vec<ActionTrace>) -> Result<AgentState, Error> {
        let schema = state.get_schema();
        let Pdl { messages, .. } = parse_pdl(
            &self.get_prompt("agent")?,
            &into_context(&state)?,
            "/",  // TODO: `<|media|>` is not supported for this prompt
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

        state.update(response, self, action_traces).await?;

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
}
