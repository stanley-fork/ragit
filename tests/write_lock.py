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
    cargo_run(["clone", "http://ragit.baehyunsol.com/sample/git"])
    os.chdir("git")
    cargo_run(["config", "--set", "model", "dummy"])

    assert cargo_run(["ii-status"], stdout=True) != "complete"

    # Adding garbage chunks to the knowledge-base: I have to make sure that `rag ii-build` runs long enough
    for i in range(50):
        write_string(f"garbage-{i}.txt", " ".join([rand_word() for _ in range(100)]))
        cargo_run(["add", f"garbage-{i}.txt"])

    cargo_run(["build"])
    cargo_run(["config", "--set", "model", test_model])
    ii_build_process = subprocess.Popen(["cargo", "run", "--release", "--", "ii-build"])

    cargo_run(["query", "How do I see a history of a file in git?"])
    ii_build_process.wait()
