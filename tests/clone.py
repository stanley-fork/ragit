import os
import shutil
import subprocess
import time
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    rand_word,
    write_string,
)

def clone(base2_size: int = 8000):
    goto_root()
    os.chdir("crates/server")
    os.makedirs("data/test-user/repo1")

    try:
        # step 0: run a ragit-server
        server_process = subprocess.Popen(["cargo", "run", "--release"])
        os.chdir("../..")
        mk_and_cd_tmp_dir()
        os.mkdir("base")
        os.chdir("base")

        # step 1: create a local knowledge-base
        #         base 1: a small base with 3 markdown files and an image
        cargo_run(["init"])
        cargo_run(["config", "--set", "model", "dummy"])
        write_string("sample1.txt", "Will AIs replace me?")
        write_string("sample2.txt", "AI가 개발자를 대체하게 될까요?")
        write_string("sample3.md", "![sample.png](sample.png)")
        shutil.copyfile("../../tests/images/empty.png", "sample.png")
        cargo_run(["add", "sample1.txt", "sample2.txt", "sample3.md"])
        cargo_run(["build"])
        cargo_run(["check"])

        # before we push this to server, let's wait until `ragit-server` is compiled
        for _ in range(300):
            path1 = "../../crates/server/target/release/ragit-server"
            path2 = "../../crates/server/target/release/ragit-server.exe"

            if not os.path.exists(path1) and not os.path.exists(path2):
                time.sleep(1)

            else:
                break

        else:
            raise Exception("failed to compile `ragit-server`")

        # step 2: push the local knowledge-base to the server
        cargo_run(["push", "--remote=http://127.0.0.1/test-user/repo1"])

        # step 3: create another local knowledge-base
        #         base 2: a larger base with 8k markdown files
        shutil.rmtree(".ragit")
        cargo_run(["init"])
        cargo_run(["config", "--set", "model", "dummy"])
        files = []

        for i in range(base2_size):
            write_string(f"{i}.txt", " ".join([rand_word() for _ in range(20)]))
            files.append(f"{i}.txt")

        cargo_run(["add", *files])
        cargo_run(["build"])
        cargo_run(["check"])

        # step 4: push the local knowledge-base to the server
        cargo_run(["push", "--remote=http://127.0.0.1/test-user/repo2"])

        # step 5: clone and check base 1
        os.chdir("..")
        cargo_run(["clone", "http://127.0.0.1/test-user/repo1"])
        os.chdir("repo1")
        cargo_run(["check"])
        assert "sample1.txt" not in cargo_run(["tfidf", "개발자"], stdout=True)
        assert "sample2.txt" in cargo_run(["tfidf", "개발자"], stdout=True)
        assert "sample1.txt" in cargo_run(["tfidf", "replace"], stdout=True)
        assert "sample2.txt" not in cargo_run(["tfidf", "replace"], stdout=True)

        # step 6: clone and check base 2
        os.chdir("..")
        cargo_run(["clone", "http://127.0.0.1/test-user/repo2"])
        os.chdir("repo2")
        cargo_run(["check"])

    finally:
        server_process.kill()
