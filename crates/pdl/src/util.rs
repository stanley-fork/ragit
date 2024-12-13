use base64::Engine;
use crate::error::Error;

pub fn encode_base64(bytes: &[u8]) -> String {
    base64::prelude::BASE64_STANDARD.encode(bytes)
}

pub fn decode_base64(s: &str) -> Result<Vec<u8>, Error> {
    Ok(base64::prelude::BASE64_STANDARD.decode(s)?)
}
