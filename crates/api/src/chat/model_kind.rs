use crate::api_provider::ApiProvider;
use crate::error::Error;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ModelKind {
    // TODO: llama 3 70B vs llama 3.1 70B
    //       Llama3170B sounds like 3.1T model
    Llama90BGroq,
    Llama70BGroq,
    Llama11BGroq,
    Llama8BGroq,
    Llama3BGroq,
    Llama1BGroq,
    Gemma9BGroq,
    CommandRPlus,
    CommandR,
    Haiku,
    Sonnet,
    Opus,
    Gpt4O,
    Gpt4OMini,
    Phi14BOllama,

    /// for test
    /// 1. doesn't require api key
    /// 2. always returns 'dummy' to a request
    /// 3. needs no network
    Dummy,
}

const ALL_MODELS: [ModelKind; 16] = [
    ModelKind::Llama90BGroq,
    ModelKind::Llama70BGroq,
    ModelKind::Llama11BGroq,
    ModelKind::Llama8BGroq,
    ModelKind::Gemma9BGroq,
    ModelKind::Llama3BGroq,
    ModelKind::Llama1BGroq,
    ModelKind::CommandRPlus,
    ModelKind::CommandR,
    ModelKind::Haiku,
    ModelKind::Sonnet,
    ModelKind::Opus,
    ModelKind::Gpt4O,
    ModelKind::Gpt4OMini,
    ModelKind::Phi14BOllama,
    ModelKind::Dummy,
];

impl ModelKind {
    pub fn all_kinds() -> &'static [ModelKind] {
        &ALL_MODELS[..]
    }

    pub fn to_api_friendly_name(&self) -> &'static str {
        match self {
            ModelKind::Llama90BGroq => "llama-3.2-90b-vision-preview",
            ModelKind::Llama70BGroq => "llama-3.1-70b-versatile",
            ModelKind::Llama11BGroq => "llama-3.2-11b-vision-preview",
            ModelKind::Llama8BGroq => "llama-3.1-8b-instant",
            ModelKind::Gemma9BGroq => "gemma-9b-it",
            ModelKind::Llama3BGroq => "llama-3.2-3b-preview",
            ModelKind::Llama1BGroq => "llama-3.2-1b-preview",
            ModelKind::CommandRPlus => "command-r-plus",
            ModelKind::CommandR => "command-r",
            ModelKind::Haiku => "claude-3-haiku-20240307",
            ModelKind::Sonnet => "claude-3-5-sonnet-20240620",
            ModelKind::Opus => "claude-3-opus-20240229",
            ModelKind::Gpt4O => "gpt-4o",
            ModelKind::Gpt4OMini => "gpt-4o-mini",
            ModelKind::Phi14BOllama => "phi3:14b",
            ModelKind::Dummy => "dummy",
        }
    }

    pub fn to_human_friendly_name(&self) -> &'static str {
        match self {
            ModelKind::Llama90BGroq => "llama3.2-90b-groq",
            ModelKind::Llama70BGroq => "llama3.1-70b-groq",
            ModelKind::Llama11BGroq => "llama3.2-11b-groq",
            ModelKind::Llama8BGroq => "llama3.1-8b-groq",
            ModelKind::Gemma9BGroq => "gemma-9b-groq",
            ModelKind::Llama3BGroq => "llama-3.2-3b-groq",
            ModelKind::Llama1BGroq => "llama-3.2-1b-groq",
            ModelKind::CommandRPlus => "command-r-plus",
            ModelKind::CommandR => "command-r",
            ModelKind::Haiku => "claude-3-haiku",
            ModelKind::Sonnet => "claude-3-5-sonnet",
            ModelKind::Opus => "claude-3-opus",
            ModelKind::Gpt4O => "gpt-4o",
            ModelKind::Gpt4OMini => "gpt-4o-mini",
            ModelKind::Phi14BOllama => "phi-3-14b-ollama",
            ModelKind::Dummy => "dummy",
        }
    }

    pub fn explanation(&self) -> &'static str {
        "todo"
    }

    pub fn context_size(&self) -> usize {
        match self {
            ModelKind::Llama90BGroq => 131072,
            ModelKind::Llama70BGroq => 131072,
            ModelKind::Llama11BGroq => 131072,
            ModelKind::Llama8BGroq => 131072,
            ModelKind::Gemma9BGroq => 8192,
            ModelKind::Llama3BGroq => 131072,
            ModelKind::Llama1BGroq => 131072,
            ModelKind::CommandRPlus => 128000,
            ModelKind::CommandR => 128000,
            ModelKind::Haiku => 200_000,
            ModelKind::Sonnet => 200_000,
            ModelKind::Opus => 200_000,
            ModelKind::Gpt4O => 128000,
            ModelKind::Gpt4OMini => 128000,
            ModelKind::Phi14BOllama => 8192,
            ModelKind::Dummy => usize::MAX,
        }
    }

    pub fn can_read_images(&self) -> bool {
        match self {
            // NOTE: Llama 90B and Llama 11B can read images,
            //       but groq's api does not support images with system prompts
            //       for now, all the prompts in ragit has system prompts
            ModelKind::Llama90BGroq => false,
            ModelKind::Llama70BGroq => false,
            ModelKind::Llama11BGroq => false,
            ModelKind::Llama8BGroq => false,
            ModelKind::Gemma9BGroq => false,
            ModelKind::Llama3BGroq => false,
            ModelKind::Llama1BGroq => false,
            ModelKind::CommandRPlus => false,
            ModelKind::CommandR => false,
            ModelKind::Haiku => true,
            ModelKind::Sonnet => true,
            ModelKind::Opus => true,
            ModelKind::Gpt4O => true,
            ModelKind::Gpt4OMini => true,
            ModelKind::Phi14BOllama => false,
            ModelKind::Dummy => true,
        }
    }

    // when you want to set timeout, you can call this function as a default value
    // in milliseconds
    pub fn api_timeout(&self) -> u64 {
        match self {
            // groq LPUs are very fast 
            ModelKind::Llama90BGroq => 12_000,
            ModelKind::Llama70BGroq => 12_000,
            ModelKind::Llama11BGroq => 12_000,
            ModelKind::Llama8BGroq => 12_000,
            ModelKind::Gemma9BGroq => 12_000,
            ModelKind::Llama3BGroq => 12_000,
            ModelKind::Llama1BGroq => 12_000,
            ModelKind::CommandRPlus => 60_000,
            ModelKind::CommandR => 60_000,
            ModelKind::Haiku => 60_000,
            ModelKind::Sonnet => 60_000,
            ModelKind::Opus => 60_000,
            ModelKind::Gpt4O => 60_000,
            ModelKind::Gpt4OMini => 60_000,
            ModelKind::Phi14BOllama => 60_000,
            ModelKind::Dummy => u64::MAX,
        }
    }

    // TODO: there must be a config file for this, not hard-coding it
    pub fn dollars_per_1b_input_tokens(&self) -> u64 {
        match self {
            ModelKind::Llama90BGroq => 900,
            ModelKind::Llama70BGroq => 590,
            ModelKind::Llama11BGroq => 180,
            ModelKind::Llama8BGroq => 50,
            ModelKind::Gemma9BGroq => 200,
            ModelKind::Llama3BGroq => 60,
            ModelKind::Llama1BGroq => 40,
            ModelKind::CommandRPlus => 3000,
            ModelKind::CommandR => 500,
            ModelKind::Haiku => 250,
            ModelKind::Sonnet => 3000,
            ModelKind::Opus => 15000,
            ModelKind::Gpt4O => 2500,
            ModelKind::Gpt4OMini => 150,
            ModelKind::Phi14BOllama => 0,
            ModelKind::Dummy => 0,
        }
    }

    pub fn dollars_per_1b_output_tokens(&self) -> u64 {
        match self {
            ModelKind::Llama90BGroq => 900,
            ModelKind::Llama70BGroq => 790,
            ModelKind::Llama11BGroq => 180,
            ModelKind::Llama8BGroq => 80,
            ModelKind::Gemma9BGroq => 200,
            ModelKind::Llama3BGroq => 60,
            ModelKind::Llama1BGroq => 40,
            ModelKind::CommandRPlus => 15000,
            ModelKind::CommandR => 1500,
            ModelKind::Haiku => 1250,
            ModelKind::Sonnet => 15000,
            ModelKind::Opus => 75000,
            ModelKind::Gpt4O => 10000,
            ModelKind::Gpt4OMini => 600,
            ModelKind::Phi14BOllama => 0,
            ModelKind::Dummy => 0,
        }
    }

    pub fn get_api_provider(&self) -> ApiProvider {
        match self {
            ModelKind::Llama90BGroq => ApiProvider::Groq,
            ModelKind::Llama70BGroq => ApiProvider::Groq,
            ModelKind::Llama11BGroq => ApiProvider::Groq,
            ModelKind::Llama8BGroq => ApiProvider::Groq,
            ModelKind::Gemma9BGroq => ApiProvider::Groq,
            ModelKind::Llama3BGroq => ApiProvider::Groq,
            ModelKind::Llama1BGroq => ApiProvider::Groq,
            ModelKind::CommandRPlus => ApiProvider::Cohere,
            ModelKind::CommandR => ApiProvider::Cohere,
            ModelKind::Haiku => ApiProvider::Anthropic,
            ModelKind::Sonnet => ApiProvider::Anthropic,
            ModelKind::Opus => ApiProvider::Anthropic,
            ModelKind::Gpt4O => ApiProvider::OpenAi,
            ModelKind::Gpt4OMini => ApiProvider::OpenAi,
            ModelKind::Phi14BOllama => ApiProvider::Ollama,
            ModelKind::Dummy => ApiProvider::Dummy,
        }
    }
}

impl FromStr for ModelKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<ModelKind, Error> {
        let sl = s.to_ascii_lowercase();

        for model in ModelKind::all_kinds().into_iter() {
            let name = model.to_human_friendly_name().to_ascii_lowercase();

            if name == sl {
                return Ok(*model);
            }
        }

        Err(Error::InvalidModelKind(s.to_string()))
    }
}
