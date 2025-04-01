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

def parse_rm_output(args: list[str], check=False) -> Tuple[int, int]:
    output = cargo_run(["rm"] + args, check=check, stdout=True)
    first_line = output.split("\n")[0]
    staged, processed = re.search(r"removed\s*(\d+)\s*staged\s*files\s*and\s*(\d+)\s*processed\s*files", first_line).groups()
    return int(staged), int(processed)

def add_and_rm2():
    goto_root()
    mk_and_cd_tmp_dir()
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])

    write_string("sample1.txt", "sample1")
    write_string("sample2.txt", "sample2")
    os.mkdir("dir1")
    os.chdir("dir1")
    write_string("sample3.txt", "sample3")
    write_string("sample4.txt", "sample4")
    os.chdir("..")

    cargo_run(["add", "--all"])
    assert count_files() == (4, 4, 0)  # total, staged, processed
    assert cargo_run(["rm", "--all", "--processed"], check=False) != 0  # no processed files
    assert count_files() == (4, 4, 0)
    assert parse_rm_output(["--all", "--staged"]) == (4, 0)  # staged, processed
    assert count_files() == (0, 0, 0)
    cargo_run(["add", "."])
    assert count_files() == (4, 4, 0)
    cargo_run(["build"])
    assert count_files() == (4, 0, 4)
    cargo_run(["check"])

    write_string("sample5.txt", "sample5")
    cargo_run(["add", "sample5.txt"])
    assert count_files() == (5, 1, 4)
    assert parse_rm_output(["--all", "--staged"]) == (1, 0)
    assert count_files() == (4, 0, 4)
    assert cargo_run(["rm", "dir1"], check=False) != 0
    assert cargo_run(["rm", "-r", "--staged", "dir1"], check=False) != 0
    assert count_files() == (4, 0, 4)
    assert parse_rm_output(["-r", "dir1"]) == (0, 2)
    assert count_files() == (2, 0, 2)
    assert cargo_run(["rm", "--staged", "sample1.txt"], check=False) != 0
    assert count_files() == (2, 0, 2)
    assert parse_rm_output(["sample1.txt"]) == (0, 1)
    assert count_files() == (1, 0, 1)
    assert parse_rm_output(["--all"]) == (0, 1)
    assert count_files() == (0, 0, 0)

    cargo_run(["add", "--all"])
    assert count_files() == (5, 5, 0)
    os.chdir("dir1")
    assert parse_rm_output(["--all", "--staged"]) == (5, 0)
    assert count_files() == (0, 0, 0)
    os.chdir("..")

    cargo_run(["add", "--all"])
    assert count_files() == (5, 5, 0)
    os.chdir("dir1")
    assert parse_rm_output(["../sample1.txt"]) == (1, 0)
    assert count_files() == (4, 4, 0)
    assert cargo_run(["rm", "."], check=False) != 0
    assert parse_rm_output(["-r", "."]) == (2, 0)
    assert parse_rm_output(["-r", ".."]) == (2, 0)
    assert count_files() == (0, 0, 0)
    os.chdir("..")

    cargo_run(["add", "--all"])
    cargo_run(["build"])
    assert parse_rm_output(["sample1.txt", "dir1/sample3.txt"]) == (0, 2)
    cargo_run(["add", "--all"])
    assert count_files() == (5, 2, 3)
    assert parse_rm_output(["--dry-run", "--staged", "sample1.txt"]) == (1, 0)
    assert parse_rm_output(["--dry-run", "--processed", "sample2.txt"]) == (0, 1)
    assert parse_rm_output(["sample1.txt", "sample2.txt"]) == (1, 1)
    assert count_files() == (3, 1, 2)

    cargo_run(["add", "--all"])
    assert count_files() == (5, 3, 2)
    os.remove("sample1.txt")
    os.remove("dir1/sample4.txt")
    assert parse_rm_output(["--auto", "--dry-run", "--all"]) == (1, 1)
    assert parse_rm_output(["--auto", "--all"]) == (1, 1)

    cargo_run(["build"])
    assert count_files() == (3, 0, 3)
    assert cargo_run(["rm", "sample1.txt", "sample2.txt"], check=False) != 0
    assert count_files() == (3, 0, 3)
    write_string("sample1.txt", "sample1")
    cargo_run(["add", "sample1.txt"])
    assert count_files() == (4, 1, 3)
    assert parse_rm_output(["sample1.txt", "sample2.txt", "--processed"]) == (0, 1)
    assert count_files() == (3, 1, 2)
    assert parse_rm_output(["sample1.txt", "dir1/sample3.txt", "--staged"]) == (1, 0)
    assert count_files() == (2, 0, 2)

    assert parse_rm_output(["--all"]) == (0, 2)
    assert count_files() == (0, 0, 0)
    assert parse_rm_output(["--all"]) == (0, 0)
    assert count_files() == (0, 0, 0)
