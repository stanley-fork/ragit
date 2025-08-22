---
{
    "title": "ragit 0.3.2 release",
    "date": "2025-03-01",
    "author": "baehyunsol",
    "tags": ["release"]
}
---

# ragit 0.3.2 release

2025-03-01

It's mostly bug fixes, with a few new features.

Ragit had a first-ever contribution from someone other than me ([baehyunsol]). Huge thanks to [robottwo]!

[baehyunsol]: https://github.com/baehyunsol
[robottwo]: https://github.com/robottwo

## Dependencies

- chrono: 0.4.39 -> 0.4.40
- flate2: 1.0.35 -> 1.1.0
- serde: 1.0.217 -> 1.0.218
- serde_json: 1.0.135 -> 1.0.139

## ragit-server

Added 3 endpoints to ragit-server.

- `/{user-name}/{repo-name}/cat-file/{uid}`
- `/user-list`
- `/repo-list/{user-name}`

There's also been a huge refactoring. Now you can use `?` operators in handlers, instead of directly returning `Box::new(with_status(500))`.

It's a step toward ragit-hub.

## PR #10 Improve model configuration flexibility and fallback behavior

@[robottwo]

This MR enhances the model configuration system in ragit with two key improvements:

1. Custom model configuration sources - Adds support for loading model configurations from:

- RAGIT_MODEL_CONFIG environment variable
- ~/.config/ragit/models.json file
- Existing per-repository configuration

2. Automatic fallback to lowest-cost model - Ensures ragit always uses a valid model:

- When the default model is unavailable in the user's configuration
- Selects the lowest-cost model as a fallback
- Displays a warning message when fallback occurs

3. Override defaults using ~/.config/ragit/{api.json, search.json, build.json} when init'ing a new rag.

- Allows you to set different parameters as "standard" across the types of data you are working with

Benefits

- Users can maintain consistent model configurations and RAG settings across repositories
- Improves robustness by ensuring ragit always uses a valid model
- Provides clear feedback when fallback occurs

API keys are not copied to the models by default, which allows the user to share rag-indexed directories with other users, and those other users will use their own API keys (falling back to their ~/.config/ragit/models.json keys) rather than the API key of the user who created the RAG.

## fix gh issue #8 and #9

If you run, interrupt and resume `rag build` a lot of times, you might end up with a broken knowledge-base. 0.3.2 tries to avoid such state by more robust clean-up process.

If you spawn multiple `rag build` at the same time, you might end up with a broken knowledge-base. The best solution is to add a write lock file, but we're not there yet. Instead, 0.3.2 makes `rag check --recover` smarter so that it can recover from such broken knowledge-base. `rag check --recover` runs silently, so you don't have to bother calling it manually!

## tests

4 tests are added: models_init, orphan_process, server and write_lock.
