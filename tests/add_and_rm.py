import os
import re
from typing import Tuple
from utils import (
    cargo_run,
    count_files,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

def parse_add_output(flags: list[str], files: list[str], rag_check=True) -> Tuple[int, int]:
    output = cargo_run(["add"] + flags + files, stdout=True)

    if rag_check:
        cargo_run(["check"])

    first_line = output.split("\n")[0]
    staged, ignored = re.search(r"(\d+)\s*files\s*staged\,\s*(\d+)\s*files\s*ignored", first_line).groups()
    return int(staged), int(ignored)

def add_and_rm():
    goto_root()
    mk_and_cd_tmp_dir()
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["config", "--set", "sleep_after_llm_call", "0"])
    cargo_run(["config", "--set", "strict_file_reader", "true"])

    # step 1: make sure that this table is correct
    # 1. `rag add`
    # |           | processed/modified | processed/not-modified |    staged    |    new    |  n exist  |
    # |-----------|--------------------|------------------------|--------------|-----------|-----------|
    # | ignore    | ignore             | ignore                 | ignore       | ignore    | error     |
    # | n ignore  | stage              | ignore                 | ignore       | stage     | error     |

    # 2. `rag add --reject`
    # |           | processed/modified | processed/not-modified |    staged    |    new    |  n exist  |
    # |-----------|--------------------|------------------------|--------------|-----------|-----------|
    # | ignore    | error              | error                  | error        | error     | error     |
    # | n ignore  | error              | error                  | error        | stage     | error     |

    # 3. `rag add --force`
    # |           | processed/modified | processed/not-modified |    staged    |    new    |  n exist  |
    # |-----------|--------------------|------------------------|--------------|-----------|-----------|
    # | ignore    | stage              | ignore                 | ignore       | stage     | error     |
    # | n ignore  | stage              | ignore                 | ignore       | stage     | error     |
    rules = {
        None: [
            ["i", "i", "i", "i", "e"],
            ["s", "i", "i", "s", "e"],
        ],
        "--reject": [
            ["e", "e", "e", "e", "e"],
            ["e", "e", "e", "s", "e"],
        ],
        "--force": [
            ["s", "i", "i", "s", "e"],
            ["s", "i", "i", "s", "e"],
        ],
    }
    built_file_count = 0

    for i, (flag, rule) in enumerate(rules.items()):
        flags = [] if not flag else [flag]

        for j, r in enumerate(rule):
            os.mkdir(f"{i}_{j}")
            os.chdir(f"{i}_{j}")
            files = [
                "processed_modified.txt",
                "processed_not_modified.txt",
                "staged.txt",
                "new.txt",
                "n_exist.txt",
            ]
            write_string("processed_modified.txt", "hello")
            write_string("processed_not_modified.txt", "hello")
            cargo_run(["add", "processed_modified.txt", "processed_not_modified.txt"])
            cargo_run(["build"])
            write_string("processed_modified.txt", "hi")
            write_string("staged.txt", "hi")
            cargo_run(["add", "staged.txt"])
            write_string("new.txt", "hi")

            if j == 0:
                write_string("../.ragignore", f"{i}_{j}/*")

            for file, rr in zip(files, r):
                files_before = count_files()

                if rr == "e":
                    assert cargo_run(["add", *flags, file], check=False) != 0
                    assert cargo_run(["add", "--dry-run", *flags, file], check=False) != 0

                elif rr == "i":
                    assert parse_add_output(flags + ["--dry-run"], [file]) == (0, 1)
                    assert parse_add_output(flags, [file]) == (0, 1)

                elif rr == "s":
                    assert parse_add_output(flags + ["--dry-run"], [file]) == (1, 0)
                    assert parse_add_output(flags, [file]) == (1, 0)

                files_after = count_files()

                if rr == "e" or r == "i":
                    assert files_before == files_after

                elif rr == "s":
                    assert files_before != files_after

            os.chdir("..")

    # step 2: reset --soft
    cargo_run(["reset", "--soft"])
    cargo_run(["check"])
    write_string(".ragignore", "")

    total, staged, processed = count_files()
    assert (total, staged, processed) == (0, 0, 0)

    # it ignores `.ragignore` and `.ragit/*`
    staged, _ = parse_add_output(["--all"], [])
    assert staged == 24

    cargo_run(["build"])
    cargo_run(["check"])
    total, staged, processed = count_files()
    assert (total, staged, processed) == (24, 0, 24)

    # step 3: add/remove files in another directory
    write_string("1.txt", "1")
    cargo_run(["add", "1.txt"])
    os.mkdir("sub")
    os.chdir("sub")
    write_string("2.txt", "2")
    write_string("3.txt", "3")
    staged, ignored = parse_add_output([], ["2.txt"])
    assert (staged, ignored) == (1, 0)
    staged, ignored = parse_add_output([], ["./2.txt"])  # path normalization
    assert (staged, ignored) == (0, 1)

    cargo_run(["rm", "../1.txt"])
    total, staged, processed = count_files()
    assert (total, staged, processed) == (25, 1, 24)

    staged, ignored = parse_add_output([], ["../1.txt", "2.txt"])
    assert (staged, ignored) == (1, 1)

    cargo_run(["build"])
    total, staged, processed = count_files()
    assert (total, staged, processed) == (26, 0, 26)

    os.chdir("..")
    staged, ignored = parse_add_output([], ["sub/2.txt", "sub/3.txt"])
    assert (staged, ignored) == (1, 1)

    staged, ignored = parse_add_output([], ["sub"])
    assert (staged, ignored) == (0, 2)

    cargo_run(["build"])
    total, staged, processed = count_files()
    assert (total, staged, processed) == (27, 0, 27)
