import json
import os
from random import randint, random
import re
import shutil
import subprocess
from typing import Optional, Tuple

def goto_root():
    while True:
        while "Cargo.toml" not in os.listdir():
            os.chdir("..")

        with open("Cargo.toml", "r") as f:
            if "name = \"ragit\"" in f.read():
                break

            os.chdir("..")

def clean():
    goto_root()

    for d in os.listdir():
        if os.path.isdir(d) and d.startswith("__tmp_"):
            shutil.rmtree(d)

def mk_and_cd_tmp_dir(dir_name: Optional[str] = None):
    if dir_name is None:
        # let's avoid name collision
        dir_name = f"__tmp_{randint(0, 1 << 64):x}"

    if not os.path.exists(dir_name):
        os.mkdir(dir_name)

    os.chdir(dir_name)

def read_string(path: str) -> str:
    with open(path, "r") as f:
        return f.read()

def write_string(path: str, content: str):
    with open(path, "w", encoding="utf-8") as f:
        f.write(content)

# If `stdout` and `raw_output` is set, it returns `stdout: bytes`.
# If `stdout` is set, it returns `stdout: str`.
# If `stderr` and `raw_output` is set, it returns `stderr: bytes`.
# If `stderr` is set, it returns `stderr: str`.
# If `output_schema` is set, it returns a dictionary.
# If none of above are set, it returns `return_code: int`.
#
# If `check` is set, it checks whether the return code is 0 or not. If it's not 0, it raises an error.
#
# By default, all the cargo features are disabled. It doesn't respect ragit's default features (["csv"] as of now).
def cargo_run(
    args: list[str],
    features: Optional[list[str]] = None,
    timeout: Optional[float] = None,
    check: bool = True,
    stdout: bool = False,
    stderr: bool = False,
    output_schema: Optional[list[str]] = None,  # returncode | stdout | stderr
    raw_output: bool = False,
):
    add_coverage(args)
    output_schema = output_schema or []
    features = features or None  # has to ignore an empty list
    features = ["--no-default-features"] if features is None else ["--no-default-features", "--features", ",".join(features)]
    args = ["cargo", "run", "--release", *features, "--"] + args
    kwargs = {}

    kwargs["timeout"] = timeout
    kwargs["check"] = check

    if stdout or stderr or "stdout" in output_schema or "stderr" in output_schema:
        kwargs["capture_output"] = True

        if not raw_output:
            kwargs["text"] = True
            kwargs["encoding"] = "utf-8"

    result = subprocess.run(args, **kwargs)

    if output_schema != []:
        output = {}
        print(result.stdout)
        print(result.stderr)

        if "returncode" in output_schema:
            output["returncode"] = result.returncode

        if "stdout" in output_schema:
            output["stdout"] = result.stdout

        if "stderr" in output_schema:
            output["stderr"] = result.stderr

        return output

    elif stdout:
        print(result.stdout)
        print(result.stderr)
        return result.stdout

    elif stderr:
        print(result.stdout)
        print(result.stderr)
        return result.stderr

    else:
        return result.returncode

def count_files(args: Optional[list[str]] = None, extra_check: bool = True) -> Tuple[int, int, int]:
    files = cargo_run(["ls-files"] + (args or []), stdout=True)
    first_line = files.split("\n")[0]
    total, staged, processed = re.search(r"(\d+)\stotal\sfiles\,\s(\d+)\sstaged\sfiles\,\s(\d+)\sprocessed\sfiles", first_line).groups()
    total, staged, processed = int(total), int(staged), int(processed)

    if extra_check:
        files = cargo_run(["ls-files", "--json", "--stat-only"] + (args or []), stdout=True)
        files = json.loads(files.strip())
        total_, staged_, processed_ = files["total files"], files["staged files"], files["processed files"]
        assert (total, staged, processed) == (total_, staged_, processed_)

    return total, staged, processed

def count_chunks(args: Optional[list[str]] = None, extra_check: bool = True) -> int:
    chunks = cargo_run(["ls-chunks"] + (args or []), stdout=True)
    first_line = chunks.split("\n")[0]
    chunks = int(re.search(r"^(\d+)\schunks", first_line).group(1))

    if extra_check:
        chunks_ = cargo_run(["ls-chunks", "--json", "--stat-only"] + (args or []), stdout=True)
        chunks_ = json.loads(chunks_.strip())
        chunks_ = chunks_["chunks"]
        assert chunks == chunks_

    return chunks

def count_images(args: Optional[list[str]] = None, extra_check: bool = True) -> int:
    images = cargo_run(["ls-images"] + (args or []), stdout=True)
    first_line = images.split("\n")[0]
    images = int(re.search(r"^(\d+)\simages", first_line).group(1))

    if extra_check:
        images_ = cargo_run(["ls-images", "--json", "--stat-only"] + (args or []), stdout=True)
        images_ = json.loads(images_.strip())
        images_ = images_["images"]
        assert images == images_

    return images

def ls_recursive(ext: str, path: Optional[list[str]] = None) -> list[str]:
    result = []

    if path is None:
        path = []

    for f in os.listdir():
        if not os.path.islink(f) and os.path.isdir(f):
            os.chdir(f)
            result += ls_recursive(ext, path + [f])
            os.chdir("..")

        elif f.endswith(f".{ext}"):
            result.append(os.path.join(*path, f))

    return result

def rand_word(english_only: bool = False) -> str:
    if english_only or random() < 0.5:
        return "".join([chr(randint(65, 90)) for _ in range(randint(8, 16))])

    else:
        # korean character
        return "".join([chr(randint(44032, 55203)) for _ in range(randint(8, 16))])

def get_ragit_version() -> str:
    stdout = cargo_run(["version"], output_schema=["stdout", "returncode"], check=False)["stdout"].strip()
    return stdout or "unknown"

# some test outputs include escape code that erases the terminal
# such tests generate very long output and it should be cleaned
def clean_test_output(s: str) -> str:
    d = "\x1b[H\x1b[2J\x1b[3J"

    while (i := s.find(d)) != -1:
        s = s[i + len(d):]

    d = "\x1b[J"

    while (i := s.find(d)) != -1:
        s = s[i + len(d):]

    return s

def deepcopy(v):
    return eval(str(v))

def get_commit_hash():
    try:
        import subprocess
        return subprocess.run(["git", "rev-parse", "HEAD"], capture_output=True, text=True, check=True).stdout.strip()

    except Exception as e:
        return f"cannot get commit_hash: {e}"

# It's a simple coverage test. It checks what commands and options
# the test harness runs. It doesn't care about arguments of the commands.
_coverage: set[str] = set()

def add_coverage(args: list[str]):
    if len(args) == 0:
        _coverage.add("")
        return

    if args[0] == "-C":
        args = [args[2], "-C", *args[3:]]

    # intentional cli error
    if args[0].startswith("-"):
        return

    if "--" in args:
        args = args[args.index("--") + 1:]

    command = args[0]

    # aliases
    command = {
        "create-archive": "archive-create",
        "archive": "archive-create",
        "extract-archive": "archive-extract",
        "extract": "archive-extract",
        "build-ii": "ii-build",
        "reset-ii": "ii-reset",
        "rm": "remove",
    }.get(command, command)

    # sort the options for equality check
    options = sorted([arg for arg in args[1:] if arg.startswith("-")])

    # clean arg flags
    options = [
        option if "=" not in option else option[:option.index("=")] for option in options
    ]

    _coverage.add(" ".join([command, *options]))

def get_coverage() -> list[str]:
    return sorted(list(_coverage))

_message: Optional[str] = None

# `send_message`, `recv_message` and `reset_message` are very simple
# wrapers around a global value `_message: Optional[str]`. The test
# runner resets the message buffer before each test case. If a test
# case sends a message, the message is recorded in the result.
def send_message(message: str):
    global _message

    if _message is None:
        _message = message

    else:
        _message += "\n\n" + message

def recv_message() -> Optional[str]:
    return _message

def reset_message():
    global _message
    _message = None
