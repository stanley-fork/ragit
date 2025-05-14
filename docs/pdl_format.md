# PDL: Prompt Description Language

Pdl is a special file format used by ragit project to represent prompts. It allows you to

1. write pragmatic Prmopts using [tera](https://keats.github.io/tera/) template language.
2. embed image files.
3. force LLMs to output a json with a designated schema.

There are 2 ways to use pdl.

1. Use `rag pdl` command. Run `rag help pdl` to read its document.
2. Ragit's pipeline is made of pdl files. See `.ragit/prompts/` to check how pdl files are used and test new pdl files.

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

There're 2 special syntaxes in pdl that allows you to embed images: `<|media(path/to/media.png)|>` and `<|raw_media(png:Base64OfYourImageFile)|>`.

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

There are 3 cases where it doesn't parse json.

1. If the schema is `str`, it just treats the entire output as the string. It doesn't look for quotation marks, and it doesn't run the parser. You can also add constraints to `str`. For example, if the schema is `str { min: 100 }`, it makes sure that the length of the entire output is at least 100 characters.
2. If the schema is `yesno`, it makes sure that the LLM's output is either `yes` or `no`. You cannot mix it with other json schema because `yes` and `no` are not valid json values. If yes/no is all you need, `yesno` is better than `bool` because LLMs are usually better at English than json. This type is later converted to a boolean value, `serde_json::Value::Bool` in Rust.
3. If the schema is `code`, it looks for a [markdown code block](https://github.github.com/gfm/#fenced-code-blocks). This type is later converted to a string value, `serde_json::Value::String` in Rust. The string is the content of the fenced code block, without the fences.

## Examples

### An image

```
<|user|>

<|media(assets/sample_image.png)|>

What do you see in the image?
```

When you run this pdl file with a multi-modal model, the model will tell you what it sees in the image.

### A simple schema 1: boolean

```
<|schema|>

bool

<|user|>

Is Rust a strictly typed programming language? Just say "true" or "false".
```

If the model's response has no "true" or "false", it will automatically prompt the model to "just say true or false".

### A simple schema 2: yes/no

```
<|schema|>

yesno

<|user|>

Is Rust a strictly typed programming language? Just say yes or no.
```

It's like a boolean schema, but it will make the model say yes or no. It's better sometimes since most models are better at English than json.

### A simple schema 3: code

```
<|schema|>

code

<|user|>

Write me a Python code that calculates an inverse of a matrix. Please wrap your code with 3 backticks, using markdown's fenced-code-block syntax.
```

The pdl engine will try to find a fenced code block in the LLM's output. If it cannot find one, it asks the LLM to stick to the markdown syntax. If it does, the engine extracts the code, without the fences, and returns the code.

### A schema 1: array

```
<|schema|>

[int { min: 1, max: {{documents | length}} }]

<|user|>

Below is a list of documents. Choose documents that are related to {{topic}}. You can select an arbitrary number of documents. Your output has to be in a json format, an array of integers. If no documents are relevant, just give me an empty array.

{% for document in documents %}
{{loop.index}}. {{document}}
{% endfor %}
```

It's a more useful example. The pdl engine will try to find an array in the LLM's output. If there's none, it asks the LLM to provide a valid json. If there's an array but the schema is wrong, it tells the LLM what's wrong with the schema and how it should be fixed.

You can also give extra constraints. `{ min: 0, max: {{documents | length}} }` after `int` means that the integer has to be at least 0 and at most `{{documents | length}}`. Note that the Tera engine will convert `{{documents | length}}` to an actual length BEFORE the schema parser runs.

You can see many Tera values in the prompt. You have to provide `{{documents}}` and `{{topic}}` to the pdl engine. The engine will fill the blanks with proper values.

### A schema 2: object

```
<|schema|>

[{ name: string, age: integer }]{ min: {{num_students}}, max: {{num_students}} }

<|user|>

Below is a csv file of the students of Ragit Highschool. I want you to convert it to a json array, where the schema is `[{ "name": string, "age": integer }]`. Make sure that the array includes all the {{num_students}} students.

{{csv_data}}
```

This is a simple prompt that converts a csv file to a json array. It makes sure that all the elements are json objects, where each object has two fields: "name" and "age". It also checks types. I added a constraint to the array, so that it includes all the students.
