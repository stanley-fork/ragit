---
{
    "title": "ragit 0.3.3 release",
    "date": "2025-03-09",
    "author": "baehyunsol",
    "tags": ["release"]
}
---

# ragit 0.3.3 release

2025-03-09

## Dependencies

No changes

## Rust edition bump

Now all the ragit crates use rust edition 2024.

## ragit-server

Added 4 endpoints to ragit-server.

- `GET /{user-name}/{repo-name}/chat-list`
- `GET /{user-name}/{repo-name}/chat/{chat-id}`
- `POST /{user-name}/{repo-name}/chat-list`
- `POST /{user-name}/{repo-name}/chat/{chat-id}`

Now you can build a very simple chat-app with ragit-server.

## `rag audit`

Added a new command: `rag audit`. With `rag audit` you can track how much you've spent on LLMs.

## Removal of `rerank_title.pdl`

Previously, there were 2 rerankers: rerank_summary and rerank_title. rerank_title wasn't tested thoroughly and was causing a few bugs. I was also suspicious whether rerank_title is as useful as rerank_summary.

So I just removed rerank_title entirely from the pipeline. I hope it makes the ragit pipeline more efficient and easier to maintain.

## tests

2 tests are added: audit and server2.
