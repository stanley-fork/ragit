use crate::api_provider::ApiProvider;
use crate::error::Error;
use ragit_pdl::Message;
use std::io::{Read, Write, stdin, stdout};
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ModelKind {
    // TODO: llama 3 70B vs llama 3.3 70B
    //       Llama3370B sounds like 3.3T model
    Llama90BGroq,
    Llama70BGroq,
    Llama70BR1Groq,
    Llama11BGroq,
    Llama8BGroq,
    Llama3BGroq,
    Llama1BGroq,
    CommandRPlus,
    CommandR,
    Haiku,
    Sonnet,
    Opus,
    Gpt4O,
    Gpt4OMini,
    O1Preview,
    O1Mini,
    Phi14BOllama,
    DeepSeekChat,
    DeepSeekReason,

    /// for test
    /// 1. doesn't require api key
    /// 2. always returns 'dummy' to a request
    /// 3. needs no network
    Dummy,

    /// for test
    /// 1. doesn't require api key
    /// 2. copies string from stdin
    /// 3. needs no network
    Stdin,
}

const ALL_MODELS: [ModelKind; 21] = [
    ModelKind::Llama90BGroq,
    ModelKind::Llama70BGroq,
    ModelKind::Llama70BR1Groq,
    ModelKind::Llama11BGroq,
    ModelKind::Llama8BGroq,
    ModelKind::Llama3BGroq,
    ModelKind::Llama1BGroq,
    ModelKind::CommandRPlus,
    ModelKind::CommandR,
    ModelKind::Haiku,
    ModelKind::Sonnet,
    ModelKind::Opus,
    ModelKind::Gpt4O,
    ModelKind::Gpt4OMini,
    ModelKind::O1Preview,
    ModelKind::O1Mini,
    ModelKind::Phi14BOllama,
    ModelKind::DeepSeekChat,
    ModelKind::DeepSeekReason,
    ModelKind::Dummy,
    ModelKind::Stdin,
];

impl ModelKind {
    pub fn all_kinds() -> &'static [ModelKind] {
        &ALL_MODELS[..]
    }

    pub fn to_api_friendly_name(&self) -> &'static str {
        match self {
            ModelKind::Llama90BGroq => "llama-3.2-90b-vision-preview",
            ModelKind::Llama70BGroq => "llama-3.3-70b-versatile",
            ModelKind::Llama70BR1Groq => "deepseek-r1-distill-llama-70b",
            ModelKind::Llama11BGroq => "llama-3.2-11b-vision-preview",
            ModelKind::Llama8BGroq => "llama-3.1-8b-instant",
            ModelKind::Llama3BGroq => "llama-3.2-3b-preview",
            ModelKind::Llama1BGroq => "llama-3.2-1b-preview",
            ModelKind::CommandRPlus => "command-r-plus",
            ModelKind::CommandR => "command-r",
            ModelKind::Haiku => "claude-3-5-haiku-20241022",

            // NOTE: there's `sonnet-20241022`, but I don't like it
            ModelKind::Sonnet => "claude-3-5-sonnet-20240620",
            ModelKind::Opus => "claude-3-opus-20240229",
            ModelKind::Gpt4O => "gpt-4o",
            ModelKind::Gpt4OMini => "gpt-4o-mini",
            ModelKind::O1Preview => "o1-preview",
            ModelKind::O1Mini => "o1-mini",
            ModelKind::Phi14BOllama => "phi4:14b",
            ModelKind::DeepSeekChat => "deepseek-chat",
            ModelKind::DeepSeekReason => "deepseek-reasoner",
            ModelKind::Dummy => "dummy",
            ModelKind::Stdin => "stdin",
        }
    }

    pub fn to_human_friendly_name(&self) -> &'static str {
        match self {
            ModelKind::Llama90BGroq => "llama3.2-90b-groq",
            ModelKind::Llama70BGroq => "llama3.3-70b-groq",
            ModelKind::Llama70BR1Groq => "llama-70b-r1-groq",
            ModelKind::Llama11BGroq => "llama3.2-11b-groq",
            ModelKind::Llama8BGroq => "llama3.1-8b-groq",
            ModelKind::Llama3BGroq => "llama-3.2-3b-groq",
            ModelKind::Llama1BGroq => "llama-3.2-1b-groq",
            ModelKind::CommandRPlus => "command-r-plus",
            ModelKind::CommandR => "command-r",
            ModelKind::Haiku => "claude-3.5-haiku",
            ModelKind::Sonnet => "claude-3.5-sonnet",
            ModelKind::Opus => "claude-3-opus",
            ModelKind::Gpt4O => "gpt-4o",
            ModelKind::Gpt4OMini => "gpt-4o-mini",
            ModelKind::O1Preview => "o1-preview",
            ModelKind::O1Mini => "o1-mini",
            ModelKind::Phi14BOllama => "phi-4-14b-ollama",
            ModelKind::DeepSeekChat => "deepseek-v3",
            ModelKind::DeepSeekReason => "deepseek-r1",
            ModelKind::Dummy => "dummy",
            ModelKind::Stdin => "stdin",
        }
    }

    pub fn explanation(&self) -> &'static str {
        "todo"
    }

    pub fn context_size(&self) -> usize {
        match self {
            ModelKind::Llama90BGroq => 131072,
            ModelKind::Llama70BGroq => 131072,
            ModelKind::Llama70BR1Groq => 65536,
            ModelKind::Llama11BGroq => 131072,
            ModelKind::Llama8BGroq => 131072,
            ModelKind::Llama3BGroq => 131072,
            ModelKind::Llama1BGroq => 131072,
            ModelKind::CommandRPlus => 128000,
            ModelKind::CommandR => 128000,
            ModelKind::Haiku => 200_000,
            ModelKind::Sonnet => 200_000,
            ModelKind::Opus => 200_000,
            ModelKind::Gpt4O => 128000,
            ModelKind::Gpt4OMini => 128000,
            ModelKind::O1Preview => 128000,
            ModelKind::O1Mini => 128000,
            ModelKind::Phi14BOllama => 16384,
            ModelKind::DeepSeekChat => 65536,
            ModelKind::DeepSeekReason => 65536,
            ModelKind::Dummy => usize::MAX,
            ModelKind::Stdin => usize::MAX,
        }
    }

    pub fn can_read_images(&self) -> bool {
        match self {
            // NOTE: Llama 90B and Llama 11B can read images,
            //       but groq's api does not support images with system prompts
            //       for now, all the prompts in ragit has system prompts
            ModelKind::Llama90BGroq => false,
            ModelKind::Llama70BGroq => false,
            ModelKind::Llama70BR1Groq => false,
            ModelKind::Llama11BGroq => false,
            ModelKind::Llama8BGroq => false,
            ModelKind::Llama3BGroq => false,
            ModelKind::Llama1BGroq => false,
            ModelKind::CommandRPlus => false,
            ModelKind::CommandR => false,
            ModelKind::Haiku => false,  // will be added soon
            ModelKind::Sonnet => true,
            ModelKind::Opus => true,
            ModelKind::Gpt4O => true,
            ModelKind::Gpt4OMini => true,
            ModelKind::O1Preview => false,
            ModelKind::O1Mini => false,
            ModelKind::Phi14BOllama => false,
            ModelKind::DeepSeekChat => false,
            ModelKind::DeepSeekReason => false,
            ModelKind::Dummy => true,
            ModelKind::Stdin => false,
        }
    }

    // when you want to set timeout, you can call this function as a default value
    // in milliseconds
    pub fn api_timeout(&self) -> u64 {
        match self {
            // groq LPUs are very fast
            ModelKind::Llama90BGroq => 12_000,
            ModelKind::Llama70BGroq => 12_000,
            ModelKind::Llama70BR1Groq => 30_000,
            ModelKind::Llama11BGroq => 12_000,
            ModelKind::Llama8BGroq => 12_000,
            ModelKind::Llama3BGroq => 12_000,
            ModelKind::Llama1BGroq => 12_000,
            ModelKind::CommandRPlus => 60_000,
            ModelKind::CommandR => 60_000,
            ModelKind::Haiku => 60_000,
            ModelKind::Sonnet => 60_000,
            ModelKind::Opus => 60_000,
            ModelKind::Gpt4O => 60_000,
            ModelKind::Gpt4OMini => 60_000,
            ModelKind::O1Preview => 180_000,
            ModelKind::O1Mini => 180_000,
            ModelKind::Phi14BOllama => 60_000,
            ModelKind::DeepSeekChat => 60_000,
            ModelKind::DeepSeekReason => 60_000,
            ModelKind::Dummy => u64::MAX,
            ModelKind::Stdin => u64::MAX,
        }
    }

    // TODO: there must be a config file for this, not hard-coding it
    pub fn dollars_per_1b_input_tokens(&self) -> u64 {
        match self {
            ModelKind::Llama90BGroq => 900,
            ModelKind::Llama70BGroq => 590,
            ModelKind::Llama70BR1Groq => 590,  // TODO: not known yet
            ModelKind::Llama11BGroq => 180,
            ModelKind::Llama8BGroq => 50,
            ModelKind::Llama3BGroq => 60,
            ModelKind::Llama1BGroq => 40,
            ModelKind::CommandRPlus => 2500,
            ModelKind::CommandR => 150,
            ModelKind::Haiku => 800,
            ModelKind::Sonnet => 3000,
            ModelKind::Opus => 15000,
            ModelKind::Gpt4O => 2500,
            ModelKind::Gpt4OMini => 150,
            ModelKind::O1Preview => 15000,
            ModelKind::O1Mini => 3000,
            ModelKind::Phi14BOllama => 0,
            ModelKind::DeepSeekChat => 270,
            ModelKind::DeepSeekReason => 550,
            ModelKind::Dummy => 0,
            ModelKind::Stdin => 0,
        }
    }

    pub fn dollars_per_1b_output_tokens(&self) -> u64 {
        match self {
            ModelKind::Llama90BGroq => 900,
            ModelKind::Llama70BGroq => 790,
            ModelKind::Llama70BR1Groq => 790,  // TODO: not known yet
            ModelKind::Llama11BGroq => 180,
            ModelKind::Llama8BGroq => 80,
            ModelKind::Llama3BGroq => 60,
            ModelKind::Llama1BGroq => 40,
            ModelKind::CommandRPlus => 10000,
            ModelKind::CommandR => 600,
            ModelKind::Haiku => 4000,
            ModelKind::Sonnet => 15000,
            ModelKind::Opus => 75000,
            ModelKind::Gpt4O => 10000,
            ModelKind::Gpt4OMini => 600,
            ModelKind::O1Preview => 60000,
            ModelKind::O1Mini => 12000,
            ModelKind::Phi14BOllama => 0,
            ModelKind::DeepSeekChat => 1100,
            ModelKind::DeepSeekReason => 2190,
            ModelKind::Dummy => 0,
            ModelKind::Stdin => 0,
        }
    }

    pub fn get_api_provider(&self) -> ApiProvider {
        match self {
            ModelKind::Llama90BGroq => ApiProvider::Groq,
            ModelKind::Llama70BGroq => ApiProvider::Groq,
            ModelKind::Llama70BR1Groq => ApiProvider::Groq,
            ModelKind::Llama11BGroq => ApiProvider::Groq,
            ModelKind::Llama8BGroq => ApiProvider::Groq,
            ModelKind::Llama3BGroq => ApiProvider::Groq,
            ModelKind::Llama1BGroq => ApiProvider::Groq,
            ModelKind::CommandRPlus => ApiProvider::Cohere,
            ModelKind::CommandR => ApiProvider::Cohere,
            ModelKind::Haiku => ApiProvider::Anthropic,
            ModelKind::Sonnet => ApiProvider::Anthropic,
            ModelKind::Opus => ApiProvider::Anthropic,
            ModelKind::Gpt4O => ApiProvider::OpenAi,
            ModelKind::Gpt4OMini => ApiProvider::OpenAi,
            ModelKind::O1Preview => ApiProvider::OpenAi,
            ModelKind::O1Mini => ApiProvider::OpenAi,
            ModelKind::Phi14BOllama => ApiProvider::Ollama,
            ModelKind::DeepSeekChat => ApiProvider::DeepSeek,
            ModelKind::DeepSeekReason => ApiProvider::DeepSeek,
            ModelKind::Dummy => ApiProvider::Dummy,
            ModelKind::Stdin => ApiProvider::Dummy,
        }
    }

    pub fn get_dummy_response(&self, messages: &[Message]) -> String {
        match self {
            ModelKind::Dummy => String::from("dummy"),
            ModelKind::Stdin => {
                for message in messages.iter() {
                    println!(
                        "<|{:?}|>\n\n{}\n\n",
                        message.role,
                        message.content.iter().map(|c| c.to_string()).collect::<Vec<String>>().join(""),
                    );
                }

                print!("<|Assistant|>\n\n>>> ");
                stdout().flush().unwrap();

                let mut s = String::new();
                stdin().read_to_string(&mut s).unwrap();
                s
            },
            _ => unreachable!(),
        }
    }
}

impl FromStr for ModelKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<ModelKind, Error> {
        let sl = s.to_ascii_lowercase();
        let mut partial_matches = vec![];

        for model in ModelKind::all_kinds().into_iter() {
            let name = model.to_human_friendly_name().to_ascii_lowercase();

            if name == sl {
                return Ok(*model);
            }

            if partial_match(&name, &sl) {
                partial_matches.push(*model);
            }
        }

        if partial_matches.len() == 1 {
            return Ok(partial_matches[0]);
        }

        Err(Error::InvalidModelKind(s.to_string()))
    }
}

fn partial_match(haystack: &str, needle: &str) -> bool {
    let h_bytes = haystack.bytes().collect::<Vec<_>>();
    let n_bytes = needle.bytes().collect::<Vec<_>>();
    let mut h_cursor = 0;
    let mut n_cursor = 0;

    while h_cursor < h_bytes.len() && n_cursor < n_bytes.len() {
        if h_bytes[h_cursor] == n_bytes[n_cursor] {
            h_cursor += 1;
            n_cursor += 1;
        }

        else {
            h_cursor += 1;
        }
    }

    n_cursor == n_bytes.len()
}

#[test]
fn model_from_str_test() {
    let samples = vec![
        ("llama-8b", Some(ModelKind::Llama8BGroq)),
        ("llama-70b", None),
        ("llama-70b-r1", Some(ModelKind::Llama70BR1Groq)),
        ("llama", None),
        ("gpt-4o", Some(ModelKind::Gpt4O)),
        ("gpt-4o-mini", Some(ModelKind::Gpt4OMini)),
        ("gpt4o-mini", Some(ModelKind::Gpt4OMini)),
        ("sonnet", Some(ModelKind::Sonnet)),
        ("gpt", None),
    ];

    for (s, model) in samples.into_iter() {
        match (s.parse::<ModelKind>(), model) {
            (Ok(m1), Some(m2)) if m1 == m2 => {},  // good
            (Err(e), None) => {},  // good
            e => panic!("s: {s:?}, e: {e:?}"),
        }
    }
}
