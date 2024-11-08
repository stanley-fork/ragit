# Prompt Engineering

There are 2 ways: easy one and hard one.

## Easy one: pdl files

Ragit uses pdl file format to manage prompts. Pdl is a plain-text format that contains a multi-turn conversation of a prompt. A pdl file goes through 2 steps before fed to LLM.

1. preprocess
  - A pdl file is a valid [tera](https://keats.github.io/tera/) template file. [tera](https://keats.github.io/tera/) is a template language, like jinja in Django.
  - Ragit calls `tera.render()` before anything else.
  - You cannot add new values to its context by modifying pdl files. In order to do that, you have to modify the source code.
2. pdl to an llm prompt
  - First, a pdl file is splitted into turns. There are 3 delimiters: `<|system|>`, `<|user|>` and `<|assistant|>`. Each delimiter marks a start of a new turn.
    - You cannot put a delimiter in a mid of a line. For example, `<|user|>Hello!<|assistant|>Hi!` is invalid. It has to be `\n<|user|>\nHello!\n<|assistant|>\nHi!`
    - A prompt not following any delimiter is treated as a system prompt. But I don't recommend you do that. I recommend you to start a pdl file with `<|system|>\n`
  - Second, a turn may contain multimedia files. There are 2 ways to insert multimedia files in a turn.
    - TODO: write document

You'll find pdl files in 2 places: your local ragit repo and ragit's git repo.

1. If you have initialized a ragit repo, you'll find pdl files in `./.ragit/prompts`. Modify the files and run `rag build` or `rag query` to see how LLM behaves differently. Make sure to `rag config --set dump_log true` so that you can see the conversations.
2. You can also find `prompts/` in ragit's git repo. This is the default value for prompts. If your local tests on your new prompts are satisfiable, please commit the new prompts.

## Hard one: modify the source code

Modifying pdl files is very limited. You cannot add/remove values to tera's context.

TODO: write document

## Testing prompt

Once you have modified prompts, you have to test it. The best way is to see how it actually works with real queries. By enabling `dump_log` option, you can see how LLMs interact with your new prompt. You'll find the logs at `.ragit/logs`.
