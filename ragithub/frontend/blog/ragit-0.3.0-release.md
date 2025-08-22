---
{
    "title": "ragit 0.3.0 release",
    "date": "2025-02-24",
    "author": "baehyunsol",
    "tags": ["release"]
}
---

# ragit 0.3.0 release

2025-02-24

## Dependencies

- bytes: new
- json: removed
- rand: 0.8.5 -> 0.9.0

## `rag clone`, `rag push`

Finally, ragit supports `push` operation. It is still a minimum viable product, but works!

In 0.2.1, `clone` naively fetches all the chunks and images, one per http request. It was very inefficient. Now it uses archive files, which is significantly more efficient.

## `rag archive-create`, `rag archive-extract`

0.3.0 supports creating/extracting archive files. You can easily create an archive of your knowledge-base using `rag archive-create`. You can share knowledge-bases even more easily with archive files. Clone/Push operations now also use archive files.

## `rag add`

Fixed some quirks. Previously, you could add files in `.ragit/` and `.git/`.

## `rag build`

Now `rag build` uses multi-process to call LLMs and build chunks. It makes LLM calls and incremental ii-build much more efficient.

## `rag retrieve-chunks`

Added a new command, which only retrieves chunks and does not ask a question. It's like `rag tfidf`, but a bit different.

|                         | `rag tfidf`                             | `rag retrieve-chunks`  |
|-------------------------|-----------------------------------------|------------------------|
| default input           | Keywords, but there's `--query` flag    | Query                  |
| extract keywords        | No                                      | Yes                    |
| rerank                  | No                                      | Yes                    |

Also, when ragit implements vector searches (someday), `rag retrieve-chunks` may use vector searches, but `rag tfidf` will still use tfidf.

## Chat models

Previously, all the models are hard-coded, and there's no way to add/remove your own models. Now, ragit uses `models.json` to manage models. If you're using an OpenAI-compatible model, Anthropic model or Cohere model, you can easily add your model to `models.json`.

CLI commands for adding/removing models are coming soon!

## CLI

Finally, ragit supports short flags and `--` flag. Instead of `rag rm --recursive`, you can use `rag rm -r`. At ragit 0.2.1, there's no way to add a file whose name starts with `"-"`. Now you can use `rag add -- --file-name` to do so.

## Safer file operations

0.3.0 implements `WriteMode::Atomic`, which tries its best to write a file atomically. It first creates a tmp file then rename the tmp file. In most file systems, a rename operation is atomic.

It reduces the error rate of `tests/many_chunks.py` and `tests/many_jobs.py` significantly.

## Ignore

Fixed a few bugs in the ignore-parser.

## Ragit-server

Implemented 5 new endpoints

- GET `/{user-name}/{repo-name}/archive-list`
- GET `/{user-name}/{repo-name}/archive/{archive-key}`
- POST `/{user-name}/{repo-name}/begin-push`
- POST `/{user-name}/{repo-name}/archive`
- POST `/{user-name}/{repo-name}/finalize-push`

These are for the new push/clone operations.

## tests

7 tests are added: add_and_rm2, ignore, archive, many_jobs, symlink, extract_keywords and migrate2.
