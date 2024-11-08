# Evaluate RAG

## logs

The most straight-forward way is to see the logs. First, run `rag config --set dump_log true` to enable logs. After that, run `rag query` command and check out `.ragit/logs` dir.

## token usage

By running `rag config --set dump_api_usage true`, you can enable api usage logs. It records the token counts. Unfortunately, there's no fancy ui for the record yet.

## manual tfidf

With `rag tfidf` command, you can test its tfidf engine.
