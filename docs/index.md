# RAGIT

RAGIT (rag-it) is a git-like software that turns your local files into a knowledge-base. The main goal of this project is to make knowledge-bases easy-to-create and easy-to-share.

```
rag init;
rag add --all;
rag build;
rag query "What makes ragit special?";
```

## Why another RAG framework?

RAGIT is very different from the other RAG frameworks.

1. It adds a title and summary to every chunks. The summaries make AIs very easy to rerank chunks.
2. It uses tfidf scores instead of vector searches. It first asks an AI to generate keywords from a query, then runs tfidf search with the keywords.
3. It supports markdown files with images.
4. It supports multi-turn queries (experimental).
5. You can clone/push knowledge-bases, like git.

## Platform support

Ragit is primarily supported on Linux (x64) and Mac (aarch64). It goes through a full test process before each release, on Linux and Mac. It is primarily developed on Linux and Mac.

Ragit works on Windows, but it's [not perfect](https://github.com/baehyunsol/ragit/issues/13).

Other than those 3 platforms, I haven't tested ragit on any platform.

## More documents

- [Build](./build.md)
- [Chunks](./chunks.md)
- [Configuration](./config.md)
- [Contribution](./contribution.md)
- [Evaluation](./eval.md)
- [Multi Turn](./multi_turn.md)
- [Pipeline](./pipeline.md)
- [Prompt Engineering](./prompt_engineering.md)
- [Quick Guide](./quick_guide.md)

## Interactive documents

```sh
cargo install ragit;
rag clone https://ragit.baehyunsol.com/sample/ragit;
cd ragit;
export GROQ_API_KEY=YOUR_API_KEY;
rag query "How do I contribute to ragit?";
```
