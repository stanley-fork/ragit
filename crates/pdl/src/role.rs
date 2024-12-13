use crate::error::Error;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PdlRole {
    User,
    System,
    Assistant,
    Schema,
}

impl From<&str> for PdlRole {
    fn from(s: &str) -> PdlRole {
        match s {
            "user" => PdlRole::User,
            "system" => PdlRole::System,
            "assistant" => PdlRole::Assistant,
            "schema" => PdlRole::Schema,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Role {
    User,
    System,
    Assistant,
}

impl Role {
    pub fn to_api_string(&self) -> &'static str {
        match self {
            Role::User => "user",
            Role::System => "system",
            Role::Assistant => "assistant",
        }
    }
}

impl FromStr for Role {
    type Err = Error;

    fn from_str(s: &str) -> Result<Role, Error> {
        match s.to_ascii_lowercase() {
            s if s == "user" => Ok(Role::User),
            s if s == "system" => Ok(Role::System),

            // for legacy cohere api
            s if s == "assistant" || s == "chatbot" => Ok(Role::Assistant),
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
            PdlRole::Schema => unreachable!(),
        }
    }
}
