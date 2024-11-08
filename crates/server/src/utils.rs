use ragit_fs::join4;

// ROOT/{user}/{repo}/.ragit
pub fn get_rag_path(user: &str, repo: &str) -> String {
    join4(
        "./data",  // TODO: make it configurable
        user,
        repo,
        ".ragit",
    ).unwrap()
}
