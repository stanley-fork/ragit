import os
from random import randint
import shutil
from subprocess import TimeoutExpired
from utils import (
    cargo_run,
    goto_root,
    ls_recursive,
    mk_and_cd_tmp_dir,
)

def many_jobs(model: str = "dummy", jobs: int = 999):
    goto_root()
    mk_and_cd_tmp_dir()
    shutil.copytree("../src", "src")
    os.chdir("src")

    if ".ragit" in os.listdir():
        cargo_run(["reset", "--hard"])

    cargo_run(["init"])
    cargo_run(["add", *ls_recursive("rs")])
    cargo_run(["config", "--set", "model", model])
    cargo_run(["config", "--set", "chunk_size", "512"])
    cargo_run(["config", "--set", "slide_len", "128"])

    for _ in range(5):
        try:
            cargo_run(["build", f"--jobs={jobs}"], timeout=2 + randint(0, 5) / 10)

        except TimeoutExpired:
            pass

    cargo_run(["build"])
    cargo_run(["check"])
