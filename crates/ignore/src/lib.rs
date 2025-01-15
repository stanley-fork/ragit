use ragit_fs::{FileError, get_relative_path, is_dir, read_dir};
use regex::Regex;

#[cfg(test)]
mod tests;

pub struct Ignore {
    patterns: Vec<IgnorePattern>,
}

impl Ignore {
    pub fn new() -> Self {
        Ignore {
            patterns: vec![],
        }
    }

    // like `.gitignore`, `.ragignore` never fails to parse
    pub fn parse(s: &str) -> Self {
        let mut patterns = vec![];

        for line in s.lines() {
            let t = line.trim();

            if t.is_empty() || t.starts_with("#") {
                continue;
            }

            patterns.push(IgnorePattern::parse(t));
        }

        Ignore { patterns }
    }

    pub fn walk_tree(&self, root_dir: &str, dir: &str) -> Result<Vec<(bool, String)>, FileError> {
        let mut result = vec![];
        self.walk_tree_worker(root_dir, dir, &mut result)?;
        Ok(result)
    }

    fn walk_tree_worker(&self, root_dir: &str, file: &str, buffer: &mut Vec<(bool, String)>) -> Result<(), FileError> {
        if is_dir(file) {
            if self.is_match(root_dir, file) {
                buffer.push((true, file.to_string()));
            }

            else {
                for entry in read_dir(file, false)? {
                    self.walk_tree_worker(root_dir, &entry, buffer)?;
                }
            }
        }

        else {
            buffer.push((self.is_match(root_dir, file), file.to_string()));
        }

        Ok(())
    }

    pub fn is_match(&self, root_dir: &str, file: &str) -> bool {
        let Ok(rel_path) = get_relative_path(&root_dir.to_string(), &file.to_string()) else { return false; };

        for pattern in self.patterns.iter() {
            if pattern.is_match(&rel_path) {
                return true;
            }
        }

        false
    }
}

#[derive(Clone, Debug)]
pub struct IgnorePattern {
    // TODO: I need a smarter implementation
    r: Regex,
}

impl IgnorePattern {
    // TODO: I need a smarter implementation
    pub fn parse(pattern: &str) -> Self {
        let mut pattern = pattern.to_string();

        if !pattern.starts_with("/") {
            pattern = format!("**/{pattern}");
        }

        let replaces = vec![
            (r"^\*\*$", r".+"),
            (r"^\*\*/", r"([^/]+/)_ast"),
            (r"/\*\*$", r"(/[^/]+)_ast"),
            (r"/\*\*/", r"/([^/]+/)_ast"),

            (r"^\*$", r"[^/]+"),
            (r"/\*$", r"/[^/]+"),
        ];

        let mut pattern = pattern.replace("_", "_und");
        pattern = pattern.replace("+", "_pls");
        pattern = pattern.replace(".", "_dot");
        pattern = pattern.replace("[", "_opn");
        pattern = pattern.replace("]", "_cls");

        for (bef, aft) in replaces.iter() {
            let bef = Regex::new(bef).unwrap();
            pattern = bef.replace_all(&pattern, *aft).to_string();
        }

        pattern = pattern.replace("_ast", "*");
        pattern = pattern.replace("_cls", "]");
        pattern = pattern.replace("_opn", "[");
        pattern = pattern.replace("_dot", "\\.");
        pattern = pattern.replace("_pls", "\\+");
        pattern = pattern.replace("_und", "_");
        IgnorePattern { r: Regex::new(&format!("^{pattern}$")).unwrap() }
    }

    // `path` must be a normalized, relative path
    pub fn is_match(&self, path: &str) -> bool {
        let mut path = path.to_string();

        // there's no reason to treat `a/b` and `a/b/` differently
        if path.len() > 1 && path.ends_with("/") {
            path = path.get(0..(path.len() - 1)).unwrap().to_string();
        }

        self.r.is_match(&path)
    }
}
