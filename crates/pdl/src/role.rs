use crate::error::Error;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PdlRole {
    User,
    System,
    Assistant,
    Schema,
    Reasoning,
}

impl From<&str> for PdlRole {
    fn from(s: &str) -> PdlRole {
        match s {
            "user" => PdlRole::User,
            "system" => PdlRole::System,
            "assistant" => PdlRole::Assistant,
            "schema" => PdlRole::Schema,
            "reasoning" => PdlRole::Reasoning,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Role {
    User,
    System,
    Assistant,
    Reasoning,
}

impl Role {
    // google api uses different terms. I'd really want it to
    // take `ApiProvider` as an input, but it cannot. It's my
    // mistake to separate ragit-api crate and ragit-pdl crate.
    pub fn to_api_string(&self, google: bool) -> &'static str {
        match self {
            Role::User => "user",
            Role::System => "system",
            Role::Assistant => if google { "model" } else { "assistant" },
            Role::Reasoning => "reasoning",
        }
    }
}

impl FromStr for Role {
    type Err = Error;

    fn from_str(s: &str) -> Result<Role, Error> {
        match s.to_ascii_lowercase().as_str() {
            "user" => Ok(Role::User),
            "system" => Ok(Role::System),
            "reasoning" => Ok(Role::Reasoning),

            "model"  // google ai
            | "chatbot"  // legacy cohere api
            | "assistant" => Ok(Role::Assistant),
            _ => Err(Error::InvalidRole(s.to_string())),
        }
    }
}

impl From<PdlRole> for Role {
    fn from(r: PdlRole) -> Role {
        match r {
            PdlRole::User => Role::User,
            PdlRole::System => Role::System,
            PdlRole::Assistant => Role::Assistant,
            PdlRole::Reasoning => Role::Reasoning,
            PdlRole::Schema => unreachable!(),
        }
    }
}
