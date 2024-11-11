# NOTE: I really want it to be reproducible, but it's too tough to do so.
#       It simulates a user pressing Ctrl+C when `rag build` is running.
#       It does so by timeout parameter of python's subprocess module, and
#       it's impossible to interrupt at exact line of code.
#
#       So my approach is
#       1. Prepare a lot of diverse test cases.
#       2. Interrupt at random timing, a lot of times.
#       3. Hope this covers all the cases.

import os
import shutil
from subprocess import TimeoutExpired
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir

def interrupts():
    goto_root()

    if not os.path.exists("sample/git"):
        raise Exception("Please run `python3 tests/load_samples.py git` before running this test.")

    mk_and_cd_tmp_dir()
    shutil.copytree("../sample/git", "git")
    os.chdir("git")

    if ".ragit" in os.listdir():
        cargo_run(["reset", "--hard"])

    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["config", "--set", "sleep_after_llm_call", "10"])
    txt_files = [file for file in os.listdir() if file.endswith(".txt")]
    cargo_run(["add", *txt_files])
    break2 = False

    while True:
        # there are 2 cases to cover:
        # 1. implicit --auto-recover invoked by `rag build`
        # 2. explicit `rag check --auto-recover`
        cargo_run(["check", "--auto-recover"])

        for _ in range(4):
            try:
                cargo_run(["build"], timeout=2.0)

            except TimeoutExpired:
                pass

            else:
                break2 = True
                break

        if break2:
            break
