import os
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

# NOTE: raising a cli error must be idempotent
def assert_cli_error(args: list[str], error_message: str = "cli error"):
    assert error_message in cargo_run(args, check=False, stderr=True)
    assert cargo_run(args, check=False) != 0

def cli():
    goto_root()
    mk_and_cd_tmp_dir()

    # subdir for testing `merge` command
    os.mkdir("base1")
    os.chdir("base1")
    cargo_run(["init"])
    os.chdir("..")

    os.mkdir("main")
    os.chdir("main")
    cargo_run(["init"])

    assert_cli_error([])  # no command: error
    assert_cli_error(["invalid-command"])
    assert_cli_error(["--invalid-command-that-looks-like-a-flag"])
    assert_cli_error(["config", "--invalid-flag"])

    # in git, args and flags can be interleaved
    # so is in ragit
    assert_cli_error(["add", "--invalid-flag"])
    assert_cli_error(["add", "--invalid-flag", "valid-file-name"])
    assert_cli_error(["add", "valid-file-name", "--invalid-flag"])

    assert_cli_error(["build", "--invalid-flag"])
    assert_cli_error(["merge", "../base1", "--prefix"])
    cargo_run(["merge", "../base1", "--prefix=prefix1"])
    cargo_run(["merge", "../base1", "--prefix", "prefix2"])

    # short flags
    cargo_run(["add", "--force", "."])
    cargo_run(["add", "-f", "."])
    assert_cli_error(["add", "-f", "--force", "."])
    assert_cli_error(["add", "-c", "."])

    write_string("--file", "file whose name starts with a dash")
    assert_cli_error(["add", "--file"])
    cargo_run(["add", "--", "--file"])

    # regression tests
    assert_cli_error(["config", "--set", "model"])
    assert_cli_error(["archive-create"])

    # suggest similar flags
    assert_cli_error(["query", "--iterative"], error_message="--interactive")
    assert_cli_error(["query", "-json"], error_message="--json")
    assert_cli_error(["query", "--josn"], error_message="--json")

    # suggest similar commands
    assert_cli_error(["qeury"], error_message="query")
    assert_cli_error(["archive-creat"], error_message="archive-create")

    # suggest similar keys
    assert_cli_error(["config", "--get", "modle"], error_message="model")
    assert_cli_error(["config", "--set", "modle", "dummy"], error_message="model")
    assert_cli_error(["config", "--get", "enable-rag"], error_message="enable_rag")

    # integer ranges
    assert_cli_error(["query", "--max-summaries=-1"], error_message="at least 0")

    # TODO: more tests...
