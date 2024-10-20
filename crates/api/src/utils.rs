use async_std::task;
use base64::Engine;
use crate::Error;
use std::time::Duration;

pub async fn download_file_from_url(url: &str) -> Result<Vec<u8>, Error> {
    let mut curr_error = Error::NoTry;
    let client = reqwest::Client::new();

    for _ in 0..3 {
        let request = client.get(url);
        let response = request.send().await;

        match response {
            Ok(response) => match response.status().as_u16() {
                200 => match response.bytes().await {
                    Ok(bytes) => {
                        return Ok(bytes.to_vec());
                    },
                    Err(e) => {
                        curr_error = Error::ReqwestError(e);
                    },
                },
                status_code => {
                    curr_error = Error::ServerError {
                        status_code,
                        body: response.text().await,
                    };
                },
            },
            Err(e) => {
                curr_error = Error::ReqwestError(e);
            },
        }

        task::sleep(Duration::from_millis(300)).await;
    }

    Err(curr_error)
}

pub fn encode_base64(bytes: &[u8]) -> String {
    base64::prelude::BASE64_STANDARD.encode(bytes)
}

pub fn decode_base64(s: &str) -> Result<Vec<u8>, Error> {
    Ok(base64::prelude::BASE64_STANDARD.decode(s)?)
}
