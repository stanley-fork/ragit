use ragit_fs::{FileError, get_relative_path, is_dir, read_dir};
use regex::Regex;
use std::str::FromStr;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct Ignore {
    patterns: Vec<Pattern>,
}

impl Ignore {
    pub fn new() -> Self {
        Ignore {
            patterns: vec![],
        }
    }

    pub fn add_line(&mut self, line: &str) {
        if !line.is_empty() && !line.starts_with("#") {
            self.patterns.push(Pattern::parse(line));
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

            patterns.push(Pattern::parse(t));
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
pub struct Pattern(Vec<PatternUnit>);

impl Pattern {
    pub fn parse(pattern: &str) -> Self {
        let mut pattern = pattern.to_string();

        // `a/b` -> `**/a/b`
        // `/a/b` -> `a/b`
        if !pattern.starts_with("/") {
            pattern = format!("**/{pattern}");
        }

        else {
            pattern = pattern.get(1..).unwrap().to_string();
        }

        // I'm not sure about this...
        if pattern.ends_with("/") {
            pattern = pattern.get(0..(pattern.len() - 1)).unwrap().to_string();
        }

        let mut result = pattern.split("/").map(|p| p.parse::<PatternUnit>().unwrap_or_else(|_| PatternUnit::Fixed(p.to_string()))).collect::<Vec<_>>();

        match result.last() {
            Some(PatternUnit::DoubleAster) => {},
            _ => {
                // `target` must match `crates/ignore/target/debug`
                result.push(PatternUnit::DoubleAster);
            },
        }

        Pattern(result)
    }

    // `path` must be a normalized, relative path
    pub fn is_match(&self, path: &str) -> bool {
        let mut path = path.to_string();

        // there's no reason to treat `a/b` and `a/b/` differently
        if path.len() > 1 && path.ends_with("/") {
            path = path.get(0..(path.len() - 1)).unwrap().to_string();
        }

        match_worker(
            self.0.clone(),
            path.split("/").map(|p| p.to_string()).collect::<Vec<_>>(),
        )
    }
}

fn match_worker(pattern: Vec<PatternUnit>, path: Vec<String>) -> bool {
    // (0, 0) means it's looking at pattern[0] and path[0].
    // if it reaches (pattern.len(), path.len()), it matches
    let mut cursors = vec![(0, 0)];

    while let Some((pattern_cursor, path_cursor)) = cursors.pop() {
        if pattern_cursor == pattern.len() && path_cursor == path.len() {
            return true;
        }

        if pattern_cursor >= pattern.len() || path_cursor >= path.len() {
            if let Some(PatternUnit::DoubleAster) = pattern.get(pattern_cursor) {
                if !cursors.contains(&(pattern_cursor + 1, path_cursor)) {
                    cursors.push((pattern_cursor + 1, path_cursor));
                }
            }

            continue;
        }

        if match_dir(&pattern[pattern_cursor], &path[path_cursor]) {
            if let PatternUnit::DoubleAster = &pattern[pattern_cursor] {
                if !cursors.contains(&(pattern_cursor, path_cursor + 1)) {
                    cursors.push((pattern_cursor, path_cursor + 1));
                }

                if !cursors.contains(&(pattern_cursor + 1, path_cursor)) {
                    cursors.push((pattern_cursor + 1, path_cursor));
                }
            }

            if !cursors.contains(&(pattern_cursor + 1, path_cursor + 1)) {
                cursors.push((pattern_cursor + 1, path_cursor + 1));
            }
        }
    }

    false
}

fn match_dir(pattern: &PatternUnit, path: &str) -> bool {
    match pattern {
        PatternUnit::DoubleAster => true,
        PatternUnit::Regex(r) => r.is_match(path),
        PatternUnit::Fixed(p) => path == p,
    }
}

#[derive(Clone, Debug)]
pub enum PatternUnit {
    DoubleAster,    // **
    Regex(Regex),   // a*
    Fixed(String),  // a
}

impl FromStr for PatternUnit {
    type Err = regex::Error;

    fn from_str(s: &str) -> Result<Self, regex::Error> {
        if s == "**" {
            Ok(PatternUnit::DoubleAster)
        }

        else if s.contains("*") || s.contains("?") || s.contains("[") {
            let s = s
                .replace(".", "\\.")
                .replace("+", "\\+")
                .replace("(", "\\(")
                .replace(")", "\\)")
                .replace("{", "\\{")
                .replace("}", "\\}")
                .replace("*", ".*")
                .replace("?", ".");

            Ok(PatternUnit::Regex(Regex::new(&format!("^{s}$"))?))
        }

        else {
            Ok(PatternUnit::Fixed(s.to_string()))
        }
    }
}
