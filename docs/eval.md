# Evaluate RAG

## logs

The most straight-forward way is to see the logs. First, run `rag config --set dump_log true` to enable logs.

Any command that uses LLM will write a log file. The log files are found in `.ragit/logs`. You can run `rag gc --logs` to remove the logs.

## token usage

By running `rag config --set dump_api_usage true`, you can enable api usage logs. It records the token counts. You can use `rag audit` command to see how much you spent using LLMs.

## manual tfidf

With `rag tfidf` command, you can test its tfidf engine.
