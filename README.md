# RAGIT

RAGIT (rag-it) is a git-like software that turns your local files into a knowledge-base. The main goal of this project is to make knowledge-bases easy to share.

It allows you to

1. create and share knowledge-bases easily
2. ask query on knowledge-bases

## Why another RAG framework?

RAGIT is very different from the other RAG frameworks. The differences make RAGIT suitable for mid-size data files (a few 100MBs I guess), but not for very big source.

1. It adds title and summary to every chunks. It makes AIs very easy to rerank chunks.
2. It DOES NOT use vector DB. Not using vector DB makes it difficult to scale to million files, but instead, it's VERY easy to share your knowledge-bases with others.
3. It calculates tf-idf score on every chunks. It must be fast enough for hundreds of thousands of chunks.

## More documents

- [Chunks](./docs/chunks.md)
- [Commands](./docs/commands.md)
- [Configuration](./docs/config.md)
- [Contribution](./docs/contribution.md)
- [Evaluation](./docs/eval.md)
- [Prompt Engineering](./docs/prompt_engineering.md)
- [Quick Guide](./docs/quick_guide.md)
