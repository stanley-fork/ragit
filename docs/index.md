# RAGIT

RAGIT (rag-it) is a git-like software that turns your local files into a knowledge-base. The main goal of this project is to make knowledge-bases easy-to-create and easy-to-share.

## Why another RAG framework?

RAGIT is very different from the other RAG frameworks.

1. It adds a title and summary to every chunks. The summaries make AIs very easy to rerank chunks.
2. It uses tfidf scores instead of vector searches. It first asks an AI to generate keywords from a query, then runs tfidf search with the keywords.
3. It supports markdown files with images.
4. It supports multi-turn queries (experimental).
5. You can clone/push knowledge-bases, like git.
  - `push` command is WIP.

## More documents

- [Build](./build.md)
- [Chunks](./chunks.md)
- [Configuration](./config.md)
- [Contribution](./contribution.md)
- [Evaluation](./eval.md)
- [Prompt Engineering](./prompt_engineering.md)
- [Quick Guide](./quick_guide.md)
