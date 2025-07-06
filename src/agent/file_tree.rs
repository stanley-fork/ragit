use std::collections::HashMap;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct FileTree {
    is_dir: bool,
    children: HashMap<String, FileTree>,
}

impl FileTree {
    pub fn root() -> Self {
        FileTree {
            is_dir: true,
            children: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        if self.is_dir {
            self.children.values().map(|f| f.len()).sum()
        }

        else {
            1
        }
    }

    pub fn is_empty(&self) -> bool {
        self.is_dir && self.children.is_empty()
    }

    pub fn to_paths(&self) -> Vec<String> {
        let mut result = vec![];

        for (k, v) in self.children.iter() {
            if v.is_dir {
                for p in v.to_paths().iter() {
                    result.push(format!("{k}/{p}"));
                }
            }

            else {
                result.push(k.to_string());
            }
        }

        result
    }

    // NOTE: There are a lot of hard-coded integers in this function. They're all
    //       just arbitrary numbers. I haven't done enough tests on them, and I
    //       have to.
    //       I don't want to make these configurable; that'd confuse
    //       new-comers, right?
    pub fn render(&self) -> String {
        let total_files: usize = self.children.values().map(|f| f.len()).sum();

        if total_files < 30 {
            let mut paths = self.to_paths();
            paths.sort();
            paths.join("\n")
        }

        else {
            let mut dirs = vec![];
            let mut excessive_dirs = 0;
            let mut files = vec![];
            let mut excessive_files = 0;

            for (k, v) in self.children.iter() {
                if v.is_dir {
                    dirs.push((k.to_string(), v.len()));
                }

                else {
                    files.push(k.to_string());
                }
            }

            dirs.sort_by_key(|(d, _)| d.to_string());
            files.sort();

            if dirs.len() > 15 {
                excessive_dirs = dirs.len() - 15;
                dirs = dirs[..15].to_vec();
            }

            if files.len() > 15 {
                excessive_files = files.len() - 15;
                files = files[..15].to_vec();
            }

            let mut lines = vec![
                dirs.iter().map(
                    |(d, f)| format!("{d}/    ({f} files in it, recursively)")
                ).collect::<Vec<_>>(),
                files,
            ].concat();

            match (excessive_dirs, excessive_files) {
                (0, 0) => {},
                (0, f) => {
                    lines.push(format!("... and {f} more files"));
                },
                (d, 0) => {
                    lines.push(format!("... and {d} more directories"));
                },
                (d, f) => {
                    lines.push(format!("... and {d} more directories and {f} more files"));
                },
            }

            lines.join("\n")
        }
    }

    pub fn insert(&mut self, path: &str) {
        let path_elements = path.split(|s| s == '/').filter(|s| !s.is_empty()).collect::<Vec<_>>();
        self.insert_worker(&path_elements);
    }

    fn insert_worker(&mut self, path_elements: &[&str]) {
        if path_elements.len() == 1 {
            self.children.insert(
                path_elements[0].to_string(),
                FileTree {
                    is_dir: false,
                    children: HashMap::new(),
                },
            );
        }

        else {
            let dir_name = &path_elements[0];

            match self.children.get_mut(*dir_name) {
                Some(f) => {
                    f.insert_worker(&path_elements[1..]);
                },
                None => {
                    let mut children = FileTree {
                        is_dir: true,
                        children: HashMap::new(),
                    };
                    children.insert_worker(&path_elements[1..]);
                    self.children.insert(
                        dir_name.to_string(),
                        children,
                    );
                },
            }
        }
    }
}
