import subprocess
import time
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, write_string

def write_lock(test_model: str):
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
