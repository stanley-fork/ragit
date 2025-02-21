# Ragit Stress Test
#
# This is not for the testsuite (`tests.py all`) cuz it takes too long
# and is not for the CI/CD pipeline.
# If you have done a performance improvement, please execute this code and
# record the result in this file (`log` at below).

import json
import os
import shutil
import subprocess
import time
from utils import cargo_run, clean, goto_root, mk_and_cd_tmp_dir, write_string

def timeit(name: str, f, result: dict):
    start = time.time()
    f()
    end = time.time()
    result[name] = int((end - start) * 1000)
    print(json.dumps(result, indent=4))

def run():
    goto_root()
    mk_and_cd_tmp_dir()

    result = {}

    # I've first tried with the linux kernel, but it's too big. (maybe later!)
    # I've second tried with the rust compiler, but `cargo run` behaves differently in the repository.
    subprocess.run(["git", "clone", "https://github.com/git/git"])
    os.chdir("git")
    subprocess.run(["git", "checkout", "757161efcca150a9a96b312d9e780a071e601a03"])  # the newest commit at the time of writing
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])

    # NOTE: as of 285b54, we don't need this line anymore. but I'll just keep it.
    write_string(".ragignore", ".git")

    timeit("add all files", lambda: cargo_run(["add", "--all"]), result)
    timeit("build without ii", lambda: cargo_run(["build"]), result)
    timeit("tfidf without ii", lambda: cargo_run(["tfidf", "file system"]), result)
    timeit("ii-build from scratch", lambda: cargo_run(["ii-build"]), result)
    timeit("tfidf with ii", lambda: cargo_run(["tfidf", "file system"]), result)

    shutil.rmtree(".ragit")
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["add", "--all"])
    cargo_run(["ii-build"])
    timeit("build with incremental ii", lambda: cargo_run(["build"]), result)
    timeit("ls-files reftable", lambda: cargo_run(["ls-files", "reftable"]), result)
    timeit("ls-files reftable/iter.c", lambda: cargo_run(["ls-files", "reftable/iter.c"]), result)
    timeit("ls-chunks reftable", lambda: cargo_run(["ls-chunks", "reftable"]), result)
    timeit("ls-chunks reftable/iter.c", lambda: cargo_run(["ls-chunks", "reftable/iter.c"]), result)

    clean()
    return result

if __name__ == "__main__":
    result = run()
    print(json.dumps(result, indent=4))

log = [
    # test run 1
    # commit d3d834
    # Apple Silicon M3 Pro
    {
        "add all files": 606,
        "build without ii": 63020,
        "tfidf without ii": 3113,
        "ii-build from scratch": 58623,
        "tfidf with ii": 887,
        "build with incremental ii": 485101,
        "ls-files reftable": 879,
        "ls-files reftable/iter.c": 440,
        "ls-chunks reftable": 436,
        "ls-chunks reftable/iter.c": 425
    },
    # test run 2
    # commit 11bcd4
    # Apple Silicon M3 Pro
    {
        "add all files": 602,
        "build without ii": 293189,
        "tfidf without ii": 2187,
        "ii-build from scratch": 61521,
        "tfidf with ii": 911,
        "build with incremental ii": 452734,
        "ls-files reftable": 932,
        "ls-files reftable/iter.c": 533,
        "ls-chunks reftable": 464,
        "ls-chunks reftable/iter.c": 504
    }
]

'''
# Memo

NOTE: The test sample has 4583 files with 17095 chunks. It's big enough for a source-code RAG, but not for a general search engine.

- test run 1: first run
  - An inverted index makes `tfidf` 3.5x faster. It's nice to see that ii is working.
  - `build with incremental ii` is terribly slow. It's 3.98x slower than `build without ii` + `ii-build from scratch`.
    - It's likely because `flush_ii_buffer` is called too frequently. It's flushed per file, which means, it's called 4583 times.
    - `ii-build from scratch` flushes only 4 times.
  - The other commands run in sub-second. It's good enough for cli users, but not for library users.
- test run 2: with multiprocess build
  - Multiprocess workers are not as good as I've expected.
  - `build without ii` takes 4.65 times longer than the first run. It's likely because of the overhead of the workers.
  - `build with incremental ii` must have gotten better, which is what I've expected, but it hasn't. It's 7% faster, but it's not enough.
'''
