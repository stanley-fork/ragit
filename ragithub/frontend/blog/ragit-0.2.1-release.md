---
{
    "title": "ragit 0.2.1 release",
    "date": "2025-02-01",
    "author": "baehyunsol",
    "tags": ["release"]
}
---

# ragit 0.2.1 release

2025-02-01

## Dependencies

- chrono: 0.4.38 -> 0.4.39
- clap: 4.5.20 -> 4.5.26
- clearscreen: 3.0.0 -> 4.0.1
- csv: new
- flate2: 1.0.34 -> 1.0.35
- futures: 0.3.30 -> 0.3.31
- image: 0.25.4 -> 0.25.5
- pathdiff: 0.2.2 -> 0.2.3
- reqwest: 0.12.9 -> 0.12.12
- serde: 1.0.214 -> 1.0.217
- serde_json: 1.0.132 -> 1.0.135
- tokio: 1.41.0 -> 1.43.0
- url: 2.5.2 -> 2.5.4

## Chat models

Added deepseek-v3 and removed gemma 9b.

Updated phi3-14b to phi4-14b.

Also added o1, deepseek-r1 and llama-70b-r1, but they're still experimental and likely to change.

Previously, you had to type a full name of a model, like `rag config --set model llama3.3-70b-groq`. But now, ragit understands short names like `rag config --set llama3.3`. It works only if the short name matches exactly 1 model.

## File readers

File readers for csv and jsonl are implemented.

## Multi-turn queries

Now it conserves more contexts for multi-turn queries.

## `rag add`

`rag add` is more git-like. It now has `--all` option and respects `.ragignore`.

## `rag merge`

`rag merge --interactive` is implemented.

## `--json`

`--json` is implemented for `rag ls-chunks`, `rag ls-files`, `rag query` and `rag tfidf`. I'm planning to implement this option for more commands.

## ragit-server

Added 2 more end points: `{user}/{repo}/chunk-count` and `{user}/{repo}/chunk-list`.

`{user}/{repo}/chunk-count` tells the number of the chunks in the repo, and `{user}/{repo}/chunk-list` gives all the chunks in the repo.

`rag clone` is not using the new end points and has to be updated.

## ragignore

`ragignore` is implemented, but it's still experimental.

## tests

Ragit now has a [ci-server](http://ragit.baehyunsol.com).

3 tests are added: clone2, csv_reader and meta.
