# Prompt Engineering

There are 2 ways: easy one and hard one.

## Easy one: pdl files

By editing pdl files, you can test different prompts. Read [pdl document](https://crates.io/crates/ragit-pdl) to learn about pdl.

You'll find pdl files in 2 places: your local ragit repo and ragit's git repo.

1. If you have initialized a ragit repo, you'll find pdl files in `./.ragit/prompts`. Modify the files and run `rag build` or `rag query` to see how LLM behaves differently. Make sure to `rag config --set dump_log true` so that you can see the conversations.
2. You can also find `prompts/` in ragit's git repo. This is the default value for prompts. If your local tests on your new prompts are satisfiable, please commit the new prompts.

## Hard one: modify the source code

Modifying pdl files is very limited. You cannot add/remove values to tera's context.

TODO: write document

## Testing prompt

Once you have modified prompts, you have to test it. The best way is to see how it actually works with real queries. By enabling `dump_log` option, you can see how LLMs interact with your new prompt. You'll find the logs at `.ragit/logs`.
