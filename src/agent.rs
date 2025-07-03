use chrono::Local;
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
use std::collections::HashSet;

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
    action_traces: Vec<ActionTrace>,

    // Actions that this agent can run.
    // It's necessary because agents have different sets of capabilities.
    // For example, the summary agent cannot run `Action::GetSummary` because
    // there's no summary yet!
    #[serde(skip)]
    actions: Vec<Action>,

    // It's generated from `actions`.
    // It's fed to AI's context.
    action_prompt: String,

    // I want to use the term `has_action_to_run`, but serde_json doesn't allow that :(
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

        else if self.has_action_to_run() {
            self.action_traces.last().unwrap().get_schema()
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
            self.action_traces.push(ActionTrace::new(self.actions.clone()));
        }

        else if self.has_action_to_run() {
            let last_action = self.action_traces.last_mut().unwrap();
            last_action.update(input, index).await?;

            match last_action.r#continue.as_ref().map(|s| s.as_str()) {
                Some("yes") => {
                    self.action_traces.push(ActionTrace::new(self.actions.clone()));
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

    fn has_action_to_run(&self) -> bool {
        if let Some(action) = self.action_traces.last() {
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
            single_paragraph: false,
            context: String::new(),
            needed_information: None,
            actions: vec![],
            action_prompt: String::new(),
            action_traces: vec![],
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
    // A set of actions that it can run.
    #[serde(skip)]
    actions: Vec<Action>,

    // It uses an index of an action instead of the action itself.
    // That's because it's tricky to (de)serialize actions.
    index: Option<usize>,
    instruction: Option<String>,
    argument: Option<String>,
    result: Option<String>,
    r#continue: Option<String>,  // "yes" | "no"
}

impl ActionTrace {
    pub fn new(actions: Vec<Action>) -> Self {
        ActionTrace {
            actions,
            ..ActionTrace::default()
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

    pub async fn update(&mut self, input: Value, index: &Index) -> Result<(), Error> {
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
            }
        }

        else if self.argument.is_none() {
            // NOTE: pdl schema `string` is infallible
            let input = input.as_str().unwrap();
            let action = self.actions[self.index.unwrap() - 1];  // AI uses 1-based index
            self.argument = Some(input.to_string());
            self.result = Some(action.run(input, index).await?);
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

impl Index {
    pub async fn agent(
        &self,
        question: &str,
        single_paragraph: bool,
        initial_context: String,

        // list of available actions
        mut actions: Vec<Action>,
    ) -> Result<String, Error> {
        // dedup
        actions = actions.into_iter().collect::<HashSet<_>>().into_iter().collect();

        // It cannot get summary if there's no summary.
        if self.get_summary().is_none() {
            actions = actions.into_iter().filter(
                |action| *action != Action::GetSummary
            ).collect();
        }

        let mut state = AgentState::default();
        state.single_paragraph = single_paragraph;
        state.question = question.to_string();
        state.context = initial_context;
        state.actions = actions.clone();
        state.action_prompt = Action::write_prompt(&actions);
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
                state.actions = actions.clone();
                state.action_prompt = Action::write_prompt(&actions);
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
}
