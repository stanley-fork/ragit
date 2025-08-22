---
{
    "title": "ragit 0.3.4 release",
    "date": "2025-03-11",
    "author": "baehyunsol",
    "tags": ["release"]
}
---

# ragit 0.3.4 release

2025-03-11

It's a hotfix of 0.3.3.

Even though it's a hotfix release, it includes some other changes. I don't have enough time and energy to manage multiple branches: all the developments are in the main branch. But no worries, I run all the tests before each release.

## Dependencies

No changes

## logs

I found logs (`.ragit/logs`) are not working at all. There was a bug in `dump_pdl()` and `dump_json()`. Now it's fixed and a regression test was added.

## ragit-server

Added 3 endpoints to ragit-server.

- `GET /{user-name}/{repo-name}/file-list`
- `GET /{user-name}/{repo-name}/search`
- `POST /{user-name}/{repo-name}/ii-build`

It also includes a massive refactoring of ragit-server.

## `rag build`

There's a small fix in `rag build`. Previously, `chunk::save_to_file` was not atomic. It lead to corruptions in some rare edge cases. Now all the `write_bytes` in `chunk::save_to_file` are atomic, so no worries!

## `rag status`

It's like `git status`. It dumps a helpful and human-readable message of the current status of the knowledge-base.

## tests

A test is added: logs
