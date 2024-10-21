# RAGIT

RAGIT (rag-it) is a git-like software that turns your local files into a knowledge-base. The main goal of this project is to make knowledge-bases easy-to-create and easy-to-share.

## Why another RAG framework?

RAGIT is very different from the other RAG frameworks. The differences make RAGIT suitable for mid-size data files (a few 100MBs I guess), but not for very big source.

1. It adds title and summary to every chunks. It makes AIs very easy to rerank chunks.
2. It uses tfidf scores instead of vector searches. It first asks an AI to generate keywords from a query, then run tfidf search with the keywords.

## More documents

- [Build](./docs/build.md)
- [Chunks](./docs/chunks.md)
- [Commands](./docs/commands.md)
- [Configuration](./docs/config.md)
- [Contribution](./docs/contribution.md)
- [Evaluation](./docs/eval.md)
- [Prompt Engineering](./docs/prompt_engineering.md)
- [Quick Guide](./docs/quick_guide.md)
