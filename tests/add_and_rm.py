import os
from utils import (
    cargo_run,
    count_files,
    goto_root,
    mk_and_cd_tmp_dir,
    parse_add_output,
    write_string,
)

def add_and_rm():
    goto_root()
    mk_and_cd_tmp_dir()
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["config", "--set", "sleep_after_llm_call", "0"])
    cargo_run(["config", "--set", "strict_file_reader", "true"])

    # step 0: you cannot build knowledge-base of `.ragit/`
    added, updated, ignored = parse_add_output([".ragit/index.json"])
    assert (added, updated, ignored) == (0, 0, 1)

    all_files = []

    # step 1: add files to a fresh knowledge-base
    for i in range(5):
        all_files.append(f"{i}.txt")
        write_string(f"{i}.txt", str(i))

    added, updated, ignored = parse_add_output(["--auto", *all_files])
    assert (added, updated, ignored) == (5, 0, 0)

    # step 1.1: --auto, --force and --ignore on the same files
    added, updated, ignored = parse_add_output(["--auto", *all_files])
    assert (added, updated, ignored) == (0, 0, 5)

    added, updated, ignored = parse_add_output(["--ignore", *all_files])
    assert (added, updated, ignored) == (0, 0, 5)

    added, updated, ignored = parse_add_output(["--force", *all_files])
    assert (added, updated, ignored) == (0, 5, 0)

    total, staged, processed = count_files()
    assert (total, staged, processed) == (5, 5, 0)

    # step 2: add files after `rag build`
    cargo_run(["build"])
    added, updated, ignored = parse_add_output(["--auto", *all_files])
    assert (added, updated, ignored) == (0, 0, 5)

    total, staged, processed = count_files()
    assert (total, staged, processed) == (5, 0, 5)

    added, updated, ignored = parse_add_output(["--force", *all_files])
    assert (added, updated, ignored) == (0, 5, 0)

    added, updated, ignored = parse_add_output(["--ignore", *all_files])
    assert (added, updated, ignored) == (0, 0, 5)

    total, staged, processed = count_files()
    assert (total, staged, processed) == (5, 5, 0)

    # step 3: add files after `rag build` and file modification
    cargo_run(["build"])
    write_string("3.txt", "100")
    added, updated, ignored = parse_add_output(["--auto", *all_files])
    assert (added, updated, ignored) == (0, 1, 4)

    added, updated, ignored = parse_add_output(["--force", *all_files])
    assert (added, updated, ignored) == (0, 5, 0)

    write_string("5.txt", "5")
    all_files.append("5.txt")

    added, updated, ignored = parse_add_output(["--ignore", *all_files])
    assert (added, updated, ignored) == (1, 0, 5)

    # step 4: rm and add files
    cargo_run(["rm", "5.txt"])
    cargo_run(["check"])
    cargo_run(["rm", "3.txt"])
    cargo_run(["check"])

    total, staged, processed = count_files()
    assert (total, staged, processed) == (4, 4, 0)

    cargo_run(["build"])
    total, staged, processed = count_files()
    assert (total, staged, processed) == (4, 0, 4)

    added, updated, ignored = parse_add_output(["--ignore", *all_files])
    assert (added, updated, ignored) == (2, 0, 4)

    total, staged, processed = count_files()
    assert (total, staged, processed) == (6, 2, 4)

    # step 5: reset --soft
    cargo_run(["reset", "--soft"])
    cargo_run(["check"])

    total, staged, processed = count_files()
    assert (total, staged, processed) == (0, 0, 0)

    added, updated, ignored = parse_add_output(["--auto", *all_files])
    assert (added, updated, ignored) == (6, 0, 0)

    cargo_run(["build"])
    cargo_run(["check"])
    total, staged, processed = count_files()
    assert (total, staged, processed) == (6, 0, 6)

    # step 6: reset --hard
    cargo_run(["reset", "--hard"])
    assert cargo_run(["check"], check=False) != 0
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    added, updated, ignored = parse_add_output(all_files)
    assert (added, updated, ignored) == (6, 0, 0)
    cargo_run(["build"])
    cargo_run(["check"])

    # step 7: add/remove files in another directory
    os.mkdir("sub")
    os.chdir("sub")
    write_string("7.txt", "7")
    write_string("8.txt", "8")
    added, updated, ignored = parse_add_output(["7.txt"])
    assert (added, updated, ignored) == (1, 0, 0)

    added, updated, ignored = parse_add_output(["--ignore", "./7.txt"])  # path normalization
    assert (added, updated, ignored) == (0, 0, 1)

    cargo_run(["rm", "../5.txt"])
    total, staged, processed = count_files()
    assert (total, staged, processed) == (6, 1, 5)

    added, updated, ignored = parse_add_output(["--ignore", "../5.txt", "7.txt"])
    assert (added, updated, ignored) == (1, 0, 1)

    cargo_run(["build"])
    total, staged, processed = count_files()
    assert (total, staged, processed) == (7, 0, 7)

    os.chdir("..")
    added, updated, ignored = parse_add_output(["--ignore", "sub/7.txt", "sub/8.txt"])
    assert (added, updated, ignored) == (1, 0, 1)

    cargo_run(["build"])
    total, staged, processed = count_files()
    assert (total, staged, processed) == (8, 0, 8)
