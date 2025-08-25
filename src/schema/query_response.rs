use super::Prettify;
use crate::chunk::Chunk;
use crate::error::Error;
use crate::query::QueryResponse;
use serde_json::Value;

pub type QueryResponseSchema = QueryResponse;

impl Prettify for QueryResponseSchema {
    fn prettify(&self) -> Result<Value, Error> {
        let mut result = serde_json::to_value(self)?;

        if let Value::Object(obj) = &mut result {
            match obj.get_mut("retrieved_chunks") {
                Some(Value::Array(retrieved_chunks)) => {
                    for retrieved_chunk in retrieved_chunks.iter_mut() {
                        let chunk = serde_json::from_value::<Chunk>(retrieved_chunk.clone())?;
                        *retrieved_chunk = chunk.prettify()?;
                    }
                },
                _ => {},
            }
        }

        Ok(result)
    }
}
