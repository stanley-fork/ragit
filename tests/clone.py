import os
from server import create_repo, create_user, health_check
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

def clone(base2_size: int = 600):
    goto_root()

    if health_check():
        raise Exception("ragit-server is already running. Please run this test in an isolated environment.")

    os.chdir("crates/server")
    os.makedirs("data/test-user/repo1")

    try:
        # step 0: run a ragit-server
        subprocess.Popen(["cargo", "run", "--release", "--", "truncate-all", "--force"])
        server_process = subprocess.Popen(["cargo", "run", "--release", "--", "run", "--force-default-config"])
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
            if health_check():
                break

            print("waiting for ragit-server to start...")
            time.sleep(1)

        else:
            raise Exception("failed to compile `ragit-server`")

        # step 2: push the local knowledge-base to the server
        create_user(name="test-user")
        create_repo(user="test-user", repo="repo1")
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
        create_repo(user="test-user", repo="repo2")
        cargo_run(["push", "--remote=http://127.0.0.1/test-user/repo2"])

        # step 5: clone samples from ragit.baehyunsol.com and push it to the local server
        os.chdir("..")

        for (base, url) in [
            ("git", "http://ragit.baehyunsol.com/sample/git"),
            ("ragit", "http://ragit.baehyunsol.com/sample/ragit"),
            ("rustc", "http://ragit.baehyunsol.com/sample/rustc"),
        ]:
            cargo_run(["clone", url], timeout=100)
            os.rename(base, base + "-cloned")
            os.chdir(base + "-cloned")
            create_repo(user="test-user", repo=base)
            cargo_run(["push", f"--remote=http://127.0.0.1/test-user/{base}"])
            os.chdir("..")

        # step 6: clone and check base 1
        cargo_run(["clone", "http://127.0.0.1/test-user/repo1"])
        os.chdir("repo1")
        cargo_run(["check"])
        assert "sample1.txt" not in cargo_run(["tfidf", "개발자"], stdout=True)
        assert "sample2.txt" in cargo_run(["tfidf", "개발자"], stdout=True)
        assert "sample1.txt" in cargo_run(["tfidf", "replace"], stdout=True)
        assert "sample2.txt" not in cargo_run(["tfidf", "replace"], stdout=True)

        # step 7: clone and check base 2
        os.chdir("..")
        cargo_run(["clone", "http://127.0.0.1/test-user/repo2"])
        os.chdir("repo2")
        cargo_run(["check"])

        # step 8: clone and check cloned bases
        os.chdir("..")

        for (base, url) in [
            ("git", "http://127.0.0.1/test-user/git"),
            ("ragit", "http://127.0.0.1/test-user/ragit"),
            ("rustc", "http://127.0.0.1/test-user/rustc"),
        ]:
            cargo_run(["clone", url])
            os.chdir(base)
            cargo_run(["check"])
            os.chdir("..")

    finally:
        server_process.kill()
