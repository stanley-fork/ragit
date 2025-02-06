import os
from random import randint, random
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

    os.chdir("crates/server")

    if os.path.exists("data/test-user"):
        shutil.rmtree("data/test-user")

def mk_and_cd_tmp_dir():
    # let's avoid name collision
    dir_name = f"__tmp_{randint(0, 1 << 64):x}"
    os.mkdir(dir_name)
    os.chdir(dir_name)

def write_string(path: str, content: str):
    with open(path, "w", encoding="utf-8") as f:
        f.write(content)

def cargo_run(
    args: list[str],
    timeout: Optional[float] = None,
    check: bool = True,
    stdout: bool = False,
    stderr: bool = False,
    output_schema: Optional[list[str]] = None,  # returncode | stdout | stderr
    raw_output: bool = False,
):
    output_schema = output_schema or []
    args = ["cargo", "run", "--release", "--"] + args
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

        if "returncode" in output_schema:
            output["returncode"] = result.returncode

        if "stdout" in output_schema:
            output["stdout"] = result.stdout

        if "stderr" in output_schema:
            output["stderr"] = result.stderr

        return output

    elif stdout:
        return result.stdout

    elif stderr:
        return result.stderr

    else:
        return result.returncode

def count_files(args: Optional[list[str]] = None) -> Tuple[int, int, int]:
    files = cargo_run(["ls-files"] + (args or []), stdout=True)
    first_line = files.split("\n")[0]
    total, staged, processed = re.search(r"(\d+)\stotal\sfiles\,\s(\d+)\sstaged\sfiles\,\s(\d+)\sprocessed\sfiles", first_line).groups()
    return int(total), int(staged), int(processed)

def count_chunks(args: Optional[list[str]] = None) -> int:
    chunks = cargo_run(["ls-chunks"] + (args or []), stdout=True)
    first_line = chunks.split("\n")[0]
    return int(re.search(r"^(\d+)\schunks", first_line).group(1))

def count_images(args: Optional[list[str]] = None) -> int:
    images = cargo_run(["ls-images"] + (args or []), stdout=True)
    first_line = images.split("\n")[0]
    return int(re.search(r"^(\d+)\simages", first_line).group(1))

def parse_tfidf_output(args: list[str]) -> int:
    output = cargo_run(["tfidf"] + args, stdout=True)

    for line in output.split("\n"):
        if (r := re.match(r"^found\s(\d+)\sresults$", line)) is not None:
            return int(r.group(1))

    raise Exception("no result found")

def ls_recursive(ext: str, path: Optional[list[str]] = None) -> list[str]:
    result = []

    if path is None:
        path = []

    for f in os.listdir():
        if os.path.isdir(f):
            os.chdir(f)
            result += ls_recursive(ext, path + [f])
            os.chdir("..")

        elif f.endswith(f".{ext}"):
            result.append(os.path.join(*path, f))

    return result

def rand_word() -> str:
    if random() < 0.5:
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

    return s

def get_commit_hash():
    try:
        import subprocess
        return subprocess.run(["git", "rev-parse", "HEAD"], capture_output=True, text=True, check=True).stdout.strip()

    except Exception as e:
        return f"cannot get commit_hash: {e}"
