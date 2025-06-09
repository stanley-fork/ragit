import json
import os
import subprocess
import time
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    rand_word,
    write_string,
)

def write_lock(test_model: str):
    # test 1: call `rag build` multiple times with different configs
    for i in range(4):
        goto_root()
        mk_and_cd_tmp_dir()
        cargo_run(["init"])
        write_string("sample1.txt", "Hello, World!")
        write_string("sample2.txt", "Hello, World!")
        cargo_run(["add", "sample1.txt", "sample2.txt"])

        if i % 2 == 0:
            cargo_run(["config", "--set", "model", "dummy"])

        else:
            cargo_run(["config", "--set", "model", test_model])

        if i // 2 == 0:
            cargo_run(["config", "--set", "sleep_after_llm_call", "3000"])

        else:
            cargo_run(["config", "--set", "sleep_after_llm_call", "0"])

        # instantiate multiple `rag build` at the same time
        for _ in range(3):
            subprocess.Popen(["cargo", "run", "--release", "--", "build"])

        cargo_run(["build"])
        time.sleep(5)  # 5 seconds would be enough... right?

        cargo_run(["check", "--recover"])
        cargo_run(["check"])

    # test 2: call `rag query` while `rag ii-build` is running
    goto_root()
    mk_and_cd_tmp_dir()
    cargo_run(["clone", "https://ragit.baehyunsol.com/sample/git"])
    os.chdir("git")
    cargo_run(["config", "--set", "model", "dummy"])

    assert cargo_run(["ii-status"], stdout=True) != "complete"

    garbage_files = []
    cargo_run(["config", "--set", "model", "dummy"])

    for i in range(200):
        garbage_file = f"garbage-{i}.txt"
        garbage_files.append(garbage_file)
        write_string(garbage_file, " ".join([rand_word() for _ in range(200)]))

    cargo_run(["add", *garbage_files])
    cargo_run(["build"])
    cargo_run(["config", "--set", "model", test_model])

    # ii-build the garbage files
    ii_build_process = subprocess.Popen(["cargo", "run", "--release", "--", "ii-build"])

    cargo_run(["query", "How do I see a history of a file in git?"])
    ii_build_process.wait()
    assert cargo_run(["ii-status"], stdout=True).strip() == "complete"

    # test 3: call multiple `rag build` multiple times
    #         this time, each will try to build ii

    # stage the garbage files
    cargo_run(["remove", *garbage_files])
    cargo_run(["add", *garbage_files])

    cargo_run(["config", "--set", "model", "dummy"])

    for _ in range(5):
        subprocess.Popen(["cargo", "run", "--release", "--", "build"])

    # wait until the build is complete
    while True:
        file_stat = json.loads(cargo_run(["ls-files", "--stat-only", "--json"], stdout=True))

        if file_stat["staged files"] == 0:
            break

        time.sleep(1)

    cargo_run(["check"])
