import shutil
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    rand_word,
    write_string,
)

# Ragit commands are not supposed to fail however stupid AIs are.
# So it'll generate a summary even with the dummy model.
def summary():
    goto_root()
    mk_and_cd_tmp_dir()

    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["config", "--set", "summary_after_build", "false"])
    uid_empty = get_uid()

    # I wish `rag summary` works with an empty knowledge-base
    cargo_run(["summary"])
    cargo_run(["check"])
    assert uid_empty != get_uid()

    write_string("a.txt", "Hello, World!")
    cargo_run(["add", "a.txt"])
    cargo_run(["build"])

    # Now that the knowledge-base was edited, the summary has to be invalidated
    assert cargo_run(["summary", "--cached"], check=False) != 0
    cargo_run(["check"])

    uid_without_summary = get_uid()

    # `summary --remove` should be nop because the summary has already been invalidated
    cargo_run(["summary", "--remove"])
    cargo_run(["check"])
    assert uid_without_summary == get_uid()

    # create a summary with the dummy model
    cargo_run(["summary"])
    cargo_run(["check"])
    assert uid_without_summary != get_uid()
    uid_with_dummy_summary = get_uid()
    dummy_summary = get_summary()

    rand_summary = rand_word()
    cargo_run(["summary", "--set", rand_summary])
    cargo_run(["check"])
    assert uid_with_dummy_summary != get_uid()
    assert dummy_summary != get_summary()

    uid_with_rand_summary = get_uid()
    assert rand_summary == get_summary()

    # it'll reuse the cached summary if there's no `--force` option
    assert uid_with_rand_summary == get_uid()
    assert rand_summary == get_summary()

    cargo_run(["summary", "--force"])
    cargo_run(["check"])
    assert uid_with_rand_summary != get_uid()
    assert rand_summary != get_summary()

    cargo_run(["summary", "--remove"])
    cargo_run(["check"])
    assert uid_without_summary == get_uid()
    assert cargo_run(["summary", "--cached"], check=False) != 0
    cargo_run(["summary", "--set", "whatever"])

    # some tests with `rag build` and summary
    cargo_run(["config", "--set", "summary_after_build", "true"])

    # an empty build: nothing happens to the summary
    cargo_run(["build"])
    cargo_run(["summary", "--cached"])

    # an empty build, but without summary: it will create a new summary
    cargo_run(["summary", "--remove"])
    cargo_run(["build"])
    cargo_run(["summary", "--cached"])

    # if there's an error in `rag build`, it will not create a summary
    write_string("b.txt", "Hello, World!!")
    shutil.copyfile("../tests/images/red.jpg", "invalid-utf8.txt")
    cargo_run(["add", "invalid-utf8.txt"])
    cargo_run(["add", "b.txt"])
    cargo_run(["build"])
    assert cargo_run(["summary", "--cached"], check=False) != 0

def get_uid():
    return cargo_run(["uid"], stdout=True).strip()

def get_summary():
    return cargo_run(["summary"], stdout=True).strip()
