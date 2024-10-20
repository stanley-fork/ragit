import os
import re
import random
import shutil
import subprocess
from subprocess import TimeoutExpired
import sys
from typing import Tuple

def goto_root():
    while "Cargo.toml" not in os.listdir() or ".gitignore" not in os.listdir():
        os.chdir("..")

def clean():
    if os.path.exists("tmp"):
        shutil.rmtree("tmp")

    goto_root()

    if os.path.exists("tmp"):
        shutil.rmtree("tmp")

# recommend you run it without `--release` flag: the code has tons of `debug_assert!`s
cargo_run = ["cargo", "run", "--"]

def count_files() -> Tuple[int, int, int]:
    files = subprocess.run([*cargo_run, "ls", "--files"], capture_output=True, text=True, check=True).stdout
    first_line = files.split("\n")[0]
    total, staged, processed = re.search(r"(\d+)\stotal\sfiles,\s(\d+)\sstaged\sfiles,\s(\d+)\sprocessed\sfiles", first_line).groups()
    return int(total), int(staged), int(processed)

def count_chunks() -> int:
    chunks = subprocess.run([*cargo_run, "ls", "--chunks"], capture_output=True, text=True, check=True).stdout
    first_line = chunks.split("\n")[0]
    return int(re.search(r"^(\d+)\schunks", first_line).group(1))

def init_to_query(test_model: str):
    goto_root()
    os.chdir("docs")
    md_files = []

    for file in os.listdir():
        if not file.endswith(".md"):
            continue

        md_files.append(file)

    if ".rag_index" in os.listdir():
        subprocess.run([*cargo_run, "reset", "--hard"], check=True)

    assert len(md_files) > 2  # `rag build` has to take at least 5 seconds
    subprocess.run([*cargo_run, "init"], check=True)
    subprocess.run([*cargo_run, "check", "--recursive"], check=True)

    # step 1: set/get config
    assert subprocess.run([*cargo_run, "config", "--set", "model", "invalid-model-name"]).returncode != 0
    subprocess.run([*cargo_run, "config", "--set", "model", "dummy"], check=True)
    subprocess.run([*cargo_run, "check", "--recursive"], check=True)
    assert "dummy" in subprocess.run([*cargo_run, "config", "--get", "model"], capture_output=True, text=True, check=True).stdout
    subprocess.run([*cargo_run, "config", "--set", "model", test_model], check=True)
    assert test_model in subprocess.run([*cargo_run, "config", "--get", "model"], capture_output=True, text=True, check=True).stdout
    subprocess.run([*cargo_run, "config", "--set", "sleep_after_llm_call", "null"], check=True)
    subprocess.run([*cargo_run, "config", "--set", "sleep_after_llm_call", "2000"], check=True)
    assert subprocess.run([*cargo_run, "config", "--set", "sleep_after_llm_call", "this_is_not_a_number"]).stdout != 0
    assert "2000" in subprocess.run([*cargo_run, "config", "--get", "sleep_after_llm_call"], capture_output=True, text=True, check=True).stdout
    subprocess.run([*cargo_run, "config", "--set", "dump_log", "true"], check=True)
    subprocess.run([*cargo_run, "check", "--recursive"], check=True)

    # step 1.1: the commands shall run anywhere inside the repo
    os.mkdir("tmp")
    os.chdir("tmp")
    assert test_model in subprocess.run([*cargo_run, "config", "--get", "model"], capture_output=True, text=True, check=True).stdout
    subprocess.run([*cargo_run, "check", "--recursive"], check=True)
    os.chdir("..")
    shutil.rmtree("tmp")

    # step 1.2: `rag config --get-all` itself has an assert statement, which checks whether there's a key collision
    subprocess.run([*cargo_run, "config", "--get-all"], check=True)

    # step 2: add the files
    subprocess.run([*cargo_run, "add", *md_files], check=True)
    subprocess.run([*cargo_run, "check", "--recursive"], check=True)
    file_count, _, _ = count_files()

    assert file_count == len(md_files)

    # step 2.1: remove all the files and add the files again
    subprocess.run([*cargo_run, "remove", *md_files], check=True)
    subprocess.run([*cargo_run, "check", "--recursive"], check=True)
    file_count, _, _ = count_files()

    assert file_count == 0

    subprocess.run([*cargo_run, "add", *md_files], check=True)
    subprocess.run([*cargo_run, "check", "--recursive"], check=True)
    file_count, _, _ = count_files()

    assert file_count == len(md_files)

    # step 3: build: pause and resume
    try:
        subprocess.run([*cargo_run, "build"], check=True, timeout=8.0 + random.randint(0, 8))

    except TimeoutExpired:
        pass

    else:
        raise Exception("The build should have timed out")

    subprocess.run([*cargo_run, "config", "--set", "sleep_after_llm_call", "null"], check=True)
    subprocess.run([*cargo_run, "build"], check=True)
    subprocess.run([*cargo_run, "check", "--recursive"], check=True)

    # running `rag build` after the knowledge-base built does nothing
    subprocess.run([*cargo_run, "build"], check=True)
    subprocess.run([*cargo_run, "check", "--recursive"], check=True)

    # step 4: ls --chunks
    chunks = subprocess.run([*cargo_run, "ls", "--chunks"], capture_output=True, text=True, check=True).stdout
    chunk_uids = []

    for line in chunks.split("\n"):
        if (r := re.match(r"^id\:\s([0-9a-f]{32,})$", line)) is not None:
            chunk_uids.append(r.group(1))

    # step 5: check whether tfidf index has token "ragit"
    has_ragit_in_tfidf = False

    for chunk_uid in chunk_uids:
        tfidf_dump = subprocess.run([*cargo_run, "tfidf", "--show", chunk_uid], capture_output=True, text=True, check=True).stdout
        has_ragit_in_tfidf = has_ragit_in_tfidf or "ragit" in tfidf_dump

    assert has_ragit_in_tfidf

    # step 6: ls commands
    file_count_prev, _, _ = count_files()
    chunk_count_prev = count_chunks()

    assert file_count_prev == len(md_files)

    # step 7: remove
    subprocess.run([*cargo_run, "remove", md_files[0]], check=True)
    subprocess.run([*cargo_run, "check", "--recursive"], check=True)
    file_count_next, _, _ = count_files()
    chunk_count_next = count_chunks()

    assert file_count_prev == file_count_next + 1
    assert chunk_count_prev > chunk_count_next

    # step 8: add again
    subprocess.run([*cargo_run, "add", md_files[0]], check=True)
    subprocess.run([*cargo_run, "check", "--recursive"], check=True)
    file_count, _, _ = count_files()
    chunk_count = count_chunks()

    assert file_count_prev == file_count
    assert chunk_count_prev > chunk_count  # `rag build` is not run yet

    subprocess.run([*cargo_run, "build"], check=True)
    subprocess.run([*cargo_run, "check", "--recursive"], check=True)
    chunk_count = count_chunks()

    assert chunk_count_prev == chunk_count

    # step 9: multiple `add` operations with different flags
    subprocess.run([*cargo_run, "add", "--ignore", md_files[0]], check=True)
    subprocess.run([*cargo_run, "check", "--recursive"], check=True)
    chunk_count_new = count_chunks()

    assert chunk_count == chunk_count_new

    subprocess.run([*cargo_run, "add", "--auto", md_files[0]], check=True)
    subprocess.run([*cargo_run, "check", "--recursive"], check=True)
    chunk_count_new = count_chunks()

    assert chunk_count == chunk_count_new

    subprocess.run([*cargo_run, "add", "--force", md_files[0]], check=True)
    subprocess.run([*cargo_run, "check", "--recursive"], check=True)
    chunk_count_new = count_chunks()

    assert chunk_count > chunk_count_new

    subprocess.run([*cargo_run, "build"], check=True)
    subprocess.run([*cargo_run, "check", "--recursive"], check=True)
    chunk_count_new = count_chunks()

    assert chunk_count == chunk_count_new

    # step 10: query
    subprocess.run([*cargo_run, "gc", "--logs"], check=True)
    subprocess.run([*cargo_run, "query", "What makes ragit special?"], check=True)

def external_bases():
    def rand_char() -> str:
        if random.random() < 0.5:
            return chr(random.randint(65, 90))

        else:
            # korean character
            return chr(random.randint(44032, 55203))

    goto_root()
    os.mkdir("tmp")
    os.chdir("tmp")
    os.mkdir("root")
    os.chdir("root")
    subprocess.run([*cargo_run, "init"], check=True)
    prefixes = {}
    base_count = random.randint(3, 8)

    for i in range(base_count):
        dir_name = f"base_{i}"
        os.mkdir(dir_name)
        os.chdir(dir_name)
        subprocess.run([*cargo_run, "init"], check=True)
        subprocess.run([*cargo_run, "check"], check=True)
        subprocess.run([*cargo_run, "config", "--set", "model", "dummy"], check=True)
        subprocess.run([*cargo_run, "config", "--set", "sleep_after_llm_call", "100"], check=True)
        subprocess.run([*cargo_run, "config", "--set", "chunk_size", "8000"], check=True)
        file_count = random.randint(3, 8)

        for j in range(file_count):
            file_name = f"doc_{j}.txt"

            with open(file_name, "w") as f:
                long_doc = "".join([rand_char() for _ in range(80_000)])
                prefix = long_doc[:16]  # let's assume it's unique
                prefixes[prefix] = (dir_name, file_name)
                f.write(long_doc)

            subprocess.run([*cargo_run, "add", "--auto", file_name], check=True)
            subprocess.run([*cargo_run, "check"], check=True)

        try:
            subprocess.run([*cargo_run, "build"], check=True, timeout=0.5)

        except TimeoutExpired:
            pass

        else:
            raise Exception("The build should have timed out")

        subprocess.run([*cargo_run, "config", "--set", "sleep_after_llm_call", "0"], check=True)
        subprocess.run([*cargo_run, "check"], check=True)
        subprocess.run([*cargo_run, "build"], check=True)
        subprocess.run([*cargo_run, "check"], check=True)
        _, _, processed_files = count_files()
        assert processed_files == file_count

        os.chdir("..")
        subprocess.run([*cargo_run, "merge", dir_name], check=True)
        subprocess.run([*cargo_run, "check", "--recursive"], check=True)

    for prefix, (dir, file) in prefixes.items():
        tfidf_result = subprocess.run([*cargo_run, "tfidf", prefix], capture_output=True, text=True, check=True).stdout
        assert dir in tfidf_result
        assert file in tfidf_result

if __name__ == "__main__":
    test_model = "dummy" if len(sys.argv) < 2 else sys.argv[1]

    try:
        init_to_query(test_model=test_model)
        external_bases()

    finally:
        clean()
