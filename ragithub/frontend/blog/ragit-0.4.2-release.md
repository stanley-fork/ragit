---
{
    "title": "ragit 0.4.2 release",
    "date": "2025-08-03",
    "author": "baehyunsol",
    "tags": ["release"]
}
---

# ragit 0.4.2 release

2025-08-03

Finally we have an agent mode!

## Dependencies

No changes

## Agent mode

You can use the agent mode with `rag query --agent <query>` command. For example, if you ask `rag query --agent "What makes ragit special?"`, the agent will browse files and chunks in the knowledge-base to answer your question.

Currently, it can only read local data in the knowledge-base. I'll add more tools, like web-searching or evaluating a python expression, later!

But, the agent mode isn't perfect yet. It's very far from perfect. It's almost always more expensive and time-consuming than non-agent mode. Also, non-agent mode is smart enough to answer most of your question. If your question is too difficult for a non-agent to answer, it would be difficult for the agent, too.

## More ergonomic inverted-index

Now `rag build` and `rag extract-archive` build an inverted index after they finish their job. So, you don't have to call `rag build-ii` manually.

There are sometimes where you have to manually build inverted-index, though. For example, if you `rag remove` a file, it makes the inverted-index dirty and you have to build the index again. I'll keep updating this feature so that you don't have to care about `rag build-ii` at all.

## `rag summary`

It calls a summary agent to create a summary of the knowledge-base. If there's a cached summary, it shows the cached one.

## `rag model`

With `rag model` command, you can fetch ai models from remote. You don't have to manually update your models.json file anymore!

The bad news is that the remote model store is not complete yet. I'm planning to add it to [this page](https://ragit.baehyunsol.com), but it might take a few weeks.

## `rag gc`

Now it supports `--all` option.

## tests

7 tests are added: cannot_read_images, erroneous_llm, fetch_models, ls_dedup, pdl_escape, server_file_tree and summary.
