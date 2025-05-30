# Ragit test suite

NOTE: This document for those who write tests, not run tests.

## Adding a new test

Let's say you want to add a test named `foo`.

First, create `foo.py` and define a function named `foo`. Usually, a test function takes no input or `test_model: str` as an input.

```py
# tests/foo.py

def foo():
    pass  # your test code here!
```

The test runner doesn't care about the return value of a function. If it raises an error, the test runner marks it "fail" and continues. If it doesn't raise anything, it's successful.

There are some utility functions used in the suite.

```py
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
)
```

- `cargo_run` is used to run ragit commands. It's short for `subprocess.run(["cargo", "run", "--release", "--", *your_args])`. For example, if you call `cargo_run(["add", "--all"])`, it's the same as running `rag add --all` in the current directory.
- A test usually runs inside a temp directory. `mk_and_cd_tmp_dir` creates and cds to a temp directory. A benefit of this function is that the test runner removes temp directories created by this function, so you don't have to worry about clean up process.
- `goto_root` literally goes to root: not `/` but where the ragit repository is cloned. It's hard to explain, but you'll find it useful.

For more information on the utility functions, please checkout `tests/utils.py`.

Once you have implemented the function `foo`, you have to add it to `tests/tests.py`. It's very easy.

1. `from foo import foo`
2. You'll see a very long string literal `help_message`. Add `foo` here.
3. A few lines later, you'll see a lot of `elif command == "test_name"`. Add `foo` here.
4. This is the most important one. You'll see a very long list `tests` after `elif command == "all"`. You have to add `foo` here so that CI runs `foo`. Please don't place your new test after `migrate`.

## Interaction between a test case and the runner

There are a few ways a test case can interact with the runner.

1. Whether the case raises an exception or not is the most important information. If it does, the test runner records the exception and its stack trace in the test result. So a failed test must raise an exception and the exception must have as much information as possible.
2. The runner doesn't care about return value at all because only successful cases return.
3. Sometimes you want information from successful tests. And sometimes it's difficult to encode all the information in the exception and stack trace. In those cases, you can use `send_message`.
  - If a case sends a message, the runner records the message in the result.
  - If it sends messages multiple times, the messages are joined with `"\n\n"`.
