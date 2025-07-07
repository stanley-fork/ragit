from utils import (
    cargo_run,
    count_chunks,
    count_files,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

def empty(test_model: str):
    goto_root()
    mk_and_cd_tmp_dir()
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", test_model])
    cargo_run(["config", "--set", "summary_after_build", "false"])
    cargo_run(["build"])
    cargo_run(["check"])
    cargo_run(["query", "what's your name?"])

    assert count_files() == (0, 0, 0)
    assert count_chunks() == 0

    write_string("empty.txt", "")
    cargo_run(["add", "empty.txt"])
    cargo_run(["build"])
    cargo_run(["check"])

    assert count_files() == (1, 0, 1)
    assert count_chunks() == 1

    cargo_run(["query", "what's your name?"])
    cargo_run(["remove", "empty.txt"])
    cargo_run(["check"])

    assert count_files() == (0, 0, 0)
    assert count_chunks() == 0

    assert "empty" in cargo_run(["summary"], stdout=True)

    # If we set metadata, it's not an empty knowledge-base.
    # A summary of an empty knowledge-base is hard-coded, so a dummy model can
    # generate one. But if it's not empty, the dummy model cannot do anything.
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["meta", "--set", "key", "value"])
    assert "empty" not in cargo_run(["summary"], stdout=True)
