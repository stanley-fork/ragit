# PDL: Prompt Description Language

Pdl is a special file format used by ragit project to represent prompts. It allows you to

1. write pragmatic Prmopts using [tera](https://keats.github.io/tera/) template language.
2. embed image files.
3. force LLMs to output a json with a designated schema.

## Language

Pdl is basically a readable format of LLM messages. For example,

```
<|user|>

Hi, what's your name?

<|assistant|>

I'm Llama.

<|user|>

How old are you?
```

is converted to

```json
[
    {
        "role": "user",
        "content": "Hi, what's your name?",
    }, {
        "role": "assistnat",
        "content": "I'm Llama",
    }, {
        "role": "user",
        "content": "How old are you?",
    },
]
```

Each turn must starts with a turn-separator: `<|user|>`, `<|assistant|>`, `<|system|>` or `<|schema|>`. A turn-separator must be following and followed by a newline character. If a content comes before any turn-separator, that's an error.

`<|schema|>` is a special type of a turn. I'll talk about it [later](#Schema).

## Template

You can write a pragmatic prompt with [tera](https://keats.github.io/tera/) template engine. When the engine parses a pdl file, the file first goes through the engine. That means [tera](https://keats.github.io/tera/) syntax is applied before any pdl syntax. You can create or remove a turn using tera syntax, or create a templated schema. You can also write comments with its syntax.

## Images

There'a special syntax in pdl that allows you to embed images.

TODO: write doc

## Schema

You can force LLMs to output a json value with a schema. You can set the schema with a `<|schema|>` turn. If it's not given, it doesn't check anything. If it's given more than once, that's an error.

```
<|schema|>

{ name: str, age: int }

<|user|>

Tell me about you.
```

The above pdl forces LLMs to output a json like `{ "name": "Llama", "age": 4 }`. It's not a magic. It's just a prompt-enhancement. So I recommend you to

1. Explain your schema in user prompt or system prompt. The `<|schema|>` turn does not reach the LLM.
2. Keep your schema simple. It works by telling the LLM which part of the output is wrong if it's wrong. It's like fixing your code with compiler error messages. If the schema is too complicated, the error message would be less readable. If it fails too much, it just returns a default value.

### Constraints

You can add constraints to schema. For example, `{ name: str, age: int { min: 0, max: 100 } }` forces the `age` value to be between 0 and 100 (both inclusive).

### Non-json schema

Basically, pdl engine first extracts json-looking string from LLM output, then parses it. For example, if the schema is a json object, the engine tries to match a curly brace using regular expression. If it fails to parse json, that's an error.

There are 2 cases where it doesn't parse json.

1. If the schema is `str`, it just treats the entire output as the string. It doesn't look for quotation marks, and it doesn't run the parser. You can also add constraints to `str`. For example, if the schema is `str { min: 100 }`, it makes sure that the length of the entire output is at least 100 characters.
2. (TODO) If the schema is `yesno`, it makes sure that the LLM's output is either `yes` or `no`. You cannot mix it with other json schema because `yes` and `no` are not valid json values. If yes/no is all you need, `yesno` is better than `bool` because LLMs are usually better at English than json.
