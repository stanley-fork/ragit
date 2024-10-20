# Contribute to Ragit

Thanks for helping Ragit! Before contributing to the project, let me remind you the goals of ragit.

1. Ragit has to be easy-to-use.
2. Knowledge-bases in Ragit has to be easy-to-share.

If you're familiar with Rust, please help us make ragit more useful. If you're not familiar with Rust, you still have tons of ways to contribute to the project. Using ragit and issuing bug reports help us a lot.

## Add new file reader

It's the easiest and the most helpful way to contribute to ragit. By adding a new file reader, ragit can process new types of files (or parse old types better).

You can find file readers in `src/index/file`. All you have to do is

1. Add a new file to `src/index/file`.
2. Define a struct and implement `FileReaderImpl` for that.
3. Add your struct to a match statement in `FileReader::new()`.
4. Run some tests.

Reading documents in the definition of `FileReaderImpl` would help you a lot.

## Prompt Engineering

See [prompt engineering.md](./prompt_engineering.md)

## Writing Documents

It also helps a lot.

## Running tests

Make sure to run tests after contributing. For now, there's only one test file: `/scripts/tests.py`.

It takes 1 input argument, the name of the model. It also requires an API key if the model needs one. Run `rag ls --models` to see the full list.

Run `python tests.py gpt-4o-mini` to run tests with gpt-4o-mini. It requires you to have env var `OPENAI_API_KEY`.

`dummy` is a special model. It doesn't require any network connection and always returns a dummy response. In order for more coverage, I recommend you to run tests both in the dummy model and a *real* model.
