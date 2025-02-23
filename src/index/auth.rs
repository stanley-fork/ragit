use super::Index;
use crate::INDEX_DIR_NAME;
use ragit_fs::{join3, read_string};

impl Index {
    /// For now, it only supports very naive way: it checks `.ragit/.auth`.
    /// The file's first line is a username and the next line is password.
    /// It returns Option<(username, Option<password>)> if available.
    pub fn auth(&self) -> Option<(String, Option<String>)> {
        let Ok(auth_path) = join3(
            &self.root_dir,
            INDEX_DIR_NAME,
            ".auth",
        ) else {
            return None
        };

        match read_string(&auth_path) {
            Ok(auth) => {
                let auth = auth.lines().collect::<Vec<_>>();

                if auth.len() < 2 {
                    None
                }

                else {
                    Some((
                        auth[0].to_string(),
                        Some(auth[1].to_string()),
                    ))
                }
            },
            Err(_) => None,
        }
    }
}
