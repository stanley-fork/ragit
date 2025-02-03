use super::Pattern;

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
                "abc/a",
            ],
            vec![
                "ab",
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
            "/a/**/b/*",
            vec![
                "a/b/b",
                "a/b/c",
                "a/b/b/c",
                "a/a/b/b",
                "a/c/b/b",
                "a/c/b/c",
            ],
            vec![
                "b/a/b/b",
                "b/a/b/c",
                "b/a/b/b/c",
                "b/a/a/b/b",
                "b/a/c/b/b",
                "b/a/c/b/c",
            ],
        ), (
            "/a/**/b",
            vec![
                "a/b/b",
                "a/b/c",
                "a/a/b/b",
                "a/b/b/c",
                "a/c/b/b",
                "a/c/b/c",
            ],
            vec![
                "b/a/b/b",
                "b/a/b/c",
                "b/a/a/b/b",
                "b/a/b/b/c",
                "b/a/c/b/b",
                "b/a/c/b/c",
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
        ), (
            "target",
            vec![
                "crates/ignore/target/debug/.fingerprint/memchr-fd54e375c9f10ea8/lib-memchr",
                "target/release/deps/liblazy_static-aa63c0c40d1aac19.rlib",
                "crates/api/target/debug/.fingerprint/adler2-927ca49ee061ac05/invoked.timestamp",
            ],
            vec![],
        ),
    ];
    let mut failures = vec![];

    for (pattern_str, matched, not_matched) in sample {
        let pattern = Pattern::parse(pattern_str);

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
