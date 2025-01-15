use super::IgnorePattern;

// TODO: find a correct gitignore-parser and make sure that these samples are correct
#[test]
fn ignore_test() {
    // (pattern, matched, not_matched)
    let sample = vec![
        (
            "abc",
            vec![
                "abc",
                "abc/",
                "a/abc",
            ],
            vec![
                "abc/a",
                "ab",

                // TODO
                //   1. I'm not sure whether gitignore allows this or not.
                //   2. `get_relative_path` will never generate a path that starts with "/".
                //   3. Then, what should we do with this case?
                "/abc",
            ],
        ), (
            "ab*",
            vec![
                "a/ab",
                "a/abc",
                "ab",
                "abc",
            ],
            vec![
                "dab",
                "aab",
            ],
        ), (
            "a/**/b",
            vec![
                "a/b", "a/b/",
                "a/b/b", "a/b/b/",
                "a/c/b", "a/c/b/",
                "a/d/d/d/b", "a/d/d/d/b/",
                "c/a/b", "c/a/b/",
                "c/a/b/b", "c/a/b/b/",
                "c/a/c/b", "c/a/c/b/",
                "c/a/d/d/d/b", "c/a/d/d/d/b/",
            ],
            vec![
                "b/a",
                "a/a",
                "b/b",
            ],
        ), (
            "*.json",
            vec![
                "a.json",
                "res/a.json",
            ],
            vec![],
        ), (
            "/*.json",
            vec![
                "a.json",
            ],
            vec![
                "a/b.json",
            ],
        ),
    ];
    let mut failures = vec![];

    for (pattern_str, matched, not_matched) in sample {
        let pattern = IgnorePattern::parse(pattern_str);

        for path in matched {
            if !pattern.is_match(path) {
                failures.push((true, pattern_str.to_string(), pattern.clone(), path.to_string()));
            }
        }

        for path in not_matched {
            if pattern.is_match(path) {
                failures.push((false, pattern_str.to_string(), pattern.clone(), path.to_string()));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "{}",
            failures.iter().map(
                |(has_to_match, pattern_str, pattern, path)| format!(
                    "(pattern: {pattern_str:?} -> {pattern:?}, path: {path:?}) {}, but {}",
                    if *has_to_match { "has to match" } else { "must not match" },
                    if *has_to_match { "but doesn't" } else { "but does" },
                )
            ).collect::<Vec<_>>().join("\n"),
        );
    }
}
