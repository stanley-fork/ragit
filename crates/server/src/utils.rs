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
