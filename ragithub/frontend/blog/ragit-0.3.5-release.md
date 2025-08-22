---
{
    "title": "ragit 0.3.5 release",
    "date": "2025-03-31",
    "author": "baehyunsol",
    "tags": ["release"]
}
---

# ragit 0.3.5 release

2025-03-31

## Dependencies

No changes

## impl super rerank

Super-rerank mode has arrived! If it's set, AI reviews more chunks before it answers your question. It takes longer time, but will give you a better result.

You can try this mode by either `rag config --set super_rerank true` or `rag query <YOUR_QUERY> --super-rerank`.

## add 2 more types to ragit-pdl's `<|schema|>`

Now it supports `yesno` type and `code` type. `yesno` type forces LLMs to just say yes or no. `code` type extracts a code block from an LLM response.

## `rag query`

It supports 8 more command line arguments: `[--max-summaries <n>]`, `[--max-retrieval <n>]`, `[--enable-ii | --disable-ii]`, `[--enable-rag | --disable-rag]`, and `[--super-rerank | --no-super-rerank]`. It overrides values in config files.

## tests

2 tests are added: config, query_options
