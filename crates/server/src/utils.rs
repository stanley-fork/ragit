use base64::Engine;
use ragit_fs::join4;

// TODO: don't unwrap this: return 500 if the user name or the repo name contains an invalid character
// ROOT/{user}/{repo}/.ragit
pub fn get_rag_path(user: &str, repo: &str) -> String {
    join4(
        "./data",  // TODO: make it configurable
        user,
        repo,
        ".ragit",
    ).unwrap()
}

pub fn decode_base64(s: &str) -> Result<Vec<u8>, ()> {
    base64::prelude::BASE64_STANDARD.decode(s).map_err(|_| ())
}
