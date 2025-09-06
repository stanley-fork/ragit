use super::{Prettify, prettify_timestamp};
use crate::constant::QUERY_HISTORY_DIR_NAME;
use crate::error::Error;
use crate::index::Index;
use crate::query::{QueryResponse, QueryTurn};
use crate::uid::Uid;
use ragit_fs::read_string;
use serde_json::Value;

pub type QueryTurnSchema = QueryTurn;

impl Index {
    pub fn get_query_schema(&self, uid: Uid) -> Result<Vec<QueryTurnSchema>, Error> {
        let query_path = Index::get_uid_path(
            &self.root_dir,
            QUERY_HISTORY_DIR_NAME,
            uid,
            Some("json"),
        )?;
        let query = read_string(&query_path)?;
        Ok(serde_json::from_str(&query)?)
    }
}

impl Prettify for QueryTurnSchema {
    fn prettify(&self) -> Result<Value, Error> {
        let mut result = serde_json::to_value(self)?;
        let uid = Uid::new_query_turn(self);

        if let Value::Object(obj) = &mut result {
            if let Some(response) = obj.get_mut("response") {
                let response_raw = serde_json::from_value::<QueryResponse>(response.clone())?;
                *response = response_raw.prettify()?;
            }

            if let Some(timestamp) = obj.get_mut("timestamp") {
                *timestamp = prettify_timestamp(timestamp);
            }

            obj.insert(String::from("uid"), uid.to_string().into());
        }

        Ok(result)
    }
}
