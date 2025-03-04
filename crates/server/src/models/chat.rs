use chrono::Local;
use crate::error::Error;
use ragit::QueryTurn;
use ragit_fs::{WriteMode, read_string, write_string};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Chat {
    pub id: String,
    pub title: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub history: Vec<QueryTurn>,
}

impl Chat {
    pub fn new(id: String) -> Self {
        Chat {
            id,
            title: None,
            created_at: Local::now().timestamp(),
            updated_at: Local::now().timestamp(),
            history: vec![],
        }
    }

    pub fn load_from_file(path: &str) -> Result<Self, Error> {
        let j = read_string(path)?;
        Ok(serde_json::from_str(&j)?)
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), Error> {
        Ok(write_string(
            path,
            &serde_json::to_string(self)?,
            WriteMode::CreateOrTruncate,
        )?)
    }
}
