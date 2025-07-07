# Configuration

Ragit is highly configurable. The config files can be found at `.ragit/configs`, but I don't recommend you modifying it manually. If you have modified it manually and have trouble accessing a knowledge-base, try `rag check --recover`.

## Global Configuration

You can set global configuration defaults by placing configuration files in `~/.config/ragit/`. When initializing a new ragit repository, it will check for the following files:

- `~/.config/ragit/api.json` - For API configuration
- `~/.config/ragit/build.json` - For build configuration
- `~/.config/ragit/query.json` - For query configuration

These files can contain a subset of the configuration fields that you want to override. You don't need to include all fields - any fields not specified will use the default values. For example, if you only want to override the `model` and `dump_log` fields in api.json, your file might look like:

```json
{
  "model": "gpt-4o",
  "dump_log": true
}
```

Any values found in these files will override the default values when creating a new repository. This allows you to have consistent configuration across all your ragit repositories.

## `config` command

A recommended way of reading/writing config is `rag config` command.

`rag config --get <KEY>` shows you a value. For example, `rag config --get model` tells you which model you're using.

`rag config --get-all` shows you all the configs.

`rag config --set <KEY> <VALUE>` allows you to set a value.

## Reference

- chunk_size: int (number of characters)
    - default: 4000
    - Ragit tries its best to make each chunk smaller than this.
    - `chunk_size` and `slide_len` isn't always perfect because ragit can handle images. It cannot divide an image into 2 pieces, so an image at the end might make a chunk bigger than `chunk_size`.
- slide_len: int (number of characters)
    - default: 1000
    - There's a sliding window between 2 chunks. Each sliding window has this length.
    - `chunk_size` and `slide_len` isn't always perfect because ragit can handle images. It cannot divide an image into 2 pieces, so an image at the end might make a chunk bigger than `chunk_size`.
- image_size: int
    - default: 2000
    - If it's 2000, ragit treats an image as 2000 characters (when calculating `chunk_size` and `slide_len`).
- min_summary_len: int (number of characters)
    - default: 200
    - Ragit uses pdl schema to force LLMs generate summaries longer than this.
- max_summary_len: int (number of characters)
    - default: 1000
- strict_file_reader: bool
    - default: false
    - It literally makes file readers more strict. For example, if there's a broken svg file, a normal file reader will treat it as a text file while a strict file will refuse to process the file.
- compression_threshold: int
    - default: 2048
- compression_level: int
    - default: 3
    - range: 0 ~ 9
- summary_after_build: bool
    - default: true
    - If it's set, it runs `rag summary` after `rag build` is complete.
- max_titles: int
    - default: 32
    - It's deprecated and not used anymore.
- max_summaries: int
    - default: 10
    - If it's 10, ragit selects 10 chunks with tfidf and reranks the 10 chunks. If there are less than 10 chunks in the knowledge-base, it doesn't run tfidf and directly reranks the chunks.
- max_retrieval: int
    - default: 3
    - If it's 3, ragit selects 3 chunks in the knowledge-base and feed that to LLM's context.
- enable_ii: bool
    - default: true
    - You can enable/disable an inverted-index. The inverted-index makes searching much faster, but the results changes very slightly.
    - It doesn't build the inverted-index. You have to run `rag ii-build` if you want to build it.
- enable_rag: bool
    - default: true
- super_rerank: bool
    - default: false
    - If it's set, it reviews more chunks. It takes much longer time, but is likely to yield better results.
    - I'm not documenting its implementation: I'll keep trying and testing new strategies.
- api_key: string
    - It's deprecated and not used anymore.
- model: string
    - Run `rag ls-models` to see the list of the models. You can also fetch new models from ragithub (WIP).
- max_retry: int
    - default: 5
    - If it's set
- timeout: int (milliseconds)
    - default: 120000
    - Timeout for API call.
- sleep_between_retries: int (milliseconds)
    - default: 15000
    - If `max_retry` is set, it sleeps this amount of time between api calls.
- sleep_after_llm_call: int (milliseconds)
    - default: null
    - If you see 429 too often, use this option. You might also want to set `--jobs=1`.
- dump_log: bool
    - default: false
    - It records EVERY api calls, including failed ones. Be careful, it would take a lot of space!
    - You can find the logs in `.ragit/logs/`
- dump_api_usage: bool
    - default: true
    - It records how many tokens and dollars are used.
