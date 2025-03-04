# Multi-turn Queries

What is a "multi-turn" query? It's one of ragit's strength, but it seems like nobody knows what the term "multi-turn" means. To be honest, I'm not sure whether I'm using the correct term. Anyway, let me explain here what "multi-turn query" is, and why it makes ragit very powerful.

I'll assume that readers have basic knowledge about RAG and ragit.

## Example

Multi-turn query is, basically asking a further question. See very simple example below.

```
<|user|>

How do I create a knowledge-base in ragit?

<|assistant|>

Based on the documents, ... blah blah

...

There are commands like `rag add`, `rag build`, and `rag ii-build`.

<|user|>

Tell me more about the `add` command.
```

It's very natural for users to ask a further question. In order to answer the second question, "Tell me more about the `add` command", ragit has to go through another RAG pipeline. But the problem is that the second question has no context. Without context, you cannot tell whether the `add` command is `git add`, `cargo add` or `rag add`. With typical RAG pipeline, you cannot retrieve chunks related to ragit from the query. If you put the entire conversation into the pipeline, the context would be contaminated.

## Solution

So, ragit implements a simple query-rewrite prompt. It asks an LLM to rephrase the user query so that the rephrased query includes all the context. Below is how the rephrased query looks like.

> Can you provide more information about the `rag add` command and how it is used to create a knowledge-base in Ragit?

Ragit runs a typical pipeline with this rephrased query!

## How to use

If you're using ragit cli, the only way to use multi-turn query is to `rag query --interactive`. There's no other handy way to use it. There must be one.

If you're building an application on top of ragit's Rust API, use [`Index::query`](https://docs.rs/ragit/latest/ragit/struct.Index.html#method.query) function. It takes outputs from the result of previous `Index::query` calls.
