use ragit_fs::join4;

// ROOT/{user}/{repo}/.rag_index
pub fn get_rag_path(user: &str, repo: &str) -> String {
    join4(
        "./data",  // TODO: make it configurable
        user,
        repo,
        ".rag_index",
    ).unwrap()
}
