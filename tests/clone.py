import os
import shutil
import subprocess
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, write_string

def clone():
    goto_root()
    os.chdir("crates/server")
    os.makedirs("data/test-user/repo1")
    # step 0: run a ragit-server
    server_process = subprocess.Popen(["cargo", "run", "--release"])
    os.chdir("../..")
    mk_and_cd_tmp_dir()

    # step 1: create a local knowledge-base
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    write_string("sample1.txt", "Will AIs replace me?")
    write_string("sample2.txt", "AI가 개발자를 대체하게 될까요?")
    write_string("sample3.md", "![sample.png](sample.png)")
    shutil.copyfile("../tests/images/empty.png", "sample.png")
    cargo_run(["add", "sample1.txt", "sample2.txt", "sample3.md"])
    cargo_run(["build"])
    cargo_run(["check"])

    # step 2: push the local knowledge-base to the server
    # sadly, `rag push` is not implemented yet
    # we have to push it manually
    shutil.copytree(".ragit", "../crates/server/data/test-user/repo1/.ragit")

    # step 3: clone and check
    cargo_run(["clone", "http://127.0.0.1/test-user/repo1"])
    os.chdir("repo1")
    cargo_run(["check"])
    assert "sample2.txt" in cargo_run(["tfidf", "개발자"], stdout=True)
    server_process.kill()
