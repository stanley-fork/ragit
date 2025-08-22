---
{
    "title": "ragit 0.4.0 release",
    "date": "2025-06-09",
    "author": "baehyunsol",
    "tags": ["release"]
}
---

# ragit 0.4.0 release

2025-06-09

Finally we have [ragithub](https://ragit.baehyunsol.com)!

## Dependencies

- csv: becomes optional
- mupdf (optional): new
- resvg (optional): new

## add support for google genai API

Now you can use gemini models!

## speedup for audit logging

Ragit records how much money it spent while building a knowledge-base. The record was implemented very inefficiently in 0.3.x. It's completely rewritten and `rag build` got much faster.

## pdf support

In 0.3.x, pdf files were partly supported. You have to run a python script before building a knowledge-base. Now it supports pdf files natively. In order to use this feature, you have to either compile with `--features=pdf` option or download a pre-built binary from github.

## svg support

File readers support svg files. In order to use this feature, you have to either compile it with `--features=svg` option or download a pre-built binary from github.

## checklist ragit-pdl schema

[https://github.com/baehyunsol/ragit/issues/17](https://github.com/baehyunsol/ragit/issues/17)

## ragit-server

Ragit-server used to be a bare-bone server where you can only push/clone repositories. Now it's a full-featured backend for [ragithub](https://ragit.baehyunsol.com)!

NOTE: It's not compatible with 0.3.x. If you want to clone something from ragithub, you have to update your ragit client.

## `rag build`

In 0.3.x, `rag build` checks the files before it actually builds the knowledge-base. If an error is found, it doesn't build the knowledge-base and returns. The check takes extra time, and was unnecessarily too strict.

Now, there's no check. It immediately begins building. If an error is found, it just skips the file and continues processing the other files. It's more efficient and ergonomic.

## `rag pdl`

A new command is added: `rag pdl`. You can run a pdl file.

## `rag pull`

A new command is added: `rag pull`. It's like `git pull`. It compares local's uid and remote's uid. If they're different it fetches the new data.

## `rag uid`

A new command is added: `rag uid`. It gives you a uid of a knowledge-base. It's like a checksum of a knowledge-base. You can easily compare knowledge-bases whether they are the same or not.

## bug fixes

A LOT.

But there're still a lot more bugs.

## tests

19 tests are added: cargo_features, clean_up_erroneous_chunks, clone_empty, generous_file_reader, gh_issue_20, korean, migrate3, outside, pdf, pdl, pull, pull_ragithub, query_with_schema, real_repos, real_repos_regression, server_ai_model, server_chat, server_permission and svg.
