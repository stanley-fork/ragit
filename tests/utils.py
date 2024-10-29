import os
from random import randint
import re
import shutil
import subprocess
from typing import Optional, Tuple

def goto_root():
    while "Cargo.toml" not in os.listdir() or ".gitignore" not in os.listdir():
        os.chdir("..")

def clean():
    for d in os.listdir():
        if os.path.isdir(d) and d.startswith("__tmp_"):
            shutil.rmtree(d)

    goto_root()

    for d in os.listdir():
        if os.path.isdir(d) and d.startswith("__tmp_"):
            shutil.rmtree(d)

def mk_and_cd_tmp_dir():
    # let's avoid name collision
    dir_name = f"__tmp_{randint(0, 1 << 64):x}"
    os.mkdir(dir_name)
    os.chdir(dir_name)

def write_string(path: str, content: str):
    with open(path, "w") as f:
        f.write(content)

def cargo_run(
    args: list[str],
    timeout: Optional[float] = None,
    check: bool = True,
    stdout: bool = False,
    stderr: bool = False,
):
    # recommend you run it without `--release` flag: the code has tons of `debug_assert!`s
    args = ["cargo", "run", "--"] + args
    kwargs = {}

    kwargs["timeout"] = timeout
    kwargs["check"] = check

    if stdout or stderr:
        kwargs["capture_output"] = True
        kwargs["text"] = True

    result = subprocess.run(args, **kwargs)

    if stdout:
        return result.stdout

    elif stderr:
        return result.stderr

    else:
        return result.returncode

def count_files() -> Tuple[int, int, int]:
    files = cargo_run(["ls-files"], stdout=True)
    first_line = files.split("\n")[0]
    total, staged, processed = re.search(r"(\d+)\stotal\sfiles\,\s(\d+)\sstaged\sfiles\,\s(\d+)\sprocessed\sfiles", first_line).groups()
    return int(total), int(staged), int(processed)

def count_chunks() -> int:
    chunks = cargo_run(["ls-chunks"], stdout=True)
    first_line = chunks.split("\n")[0]
    return int(re.search(r"^(\d+)\schunks", first_line).group(1))

def parse_add_output(args: list[str], rag_check=True) -> Tuple[int, int, int]:
    output = cargo_run(["add"] + args, stdout=True)

    if rag_check:
        cargo_run(["check", "--recursive"])

    first_line = output.split("\n")[0]
    added, updated, ignored = re.search(r"(\d+)\sadded\sfiles\,\s(\d+)\supdated\sfiles\,\s(\d+)\signored\sfiles", first_line).groups()
    return int(added), int(updated), int(ignored)
