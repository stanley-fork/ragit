import os
from server import (
    create_repo,
    create_user,
    get_api_key,
    get_repo_stat,
    spawn_ragit_server,
)
import shutil
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    rand_word,
    write_string,
)

def clone(base2_size: int = 600):
    goto_root()
    server_process = None

    try:
        server_process = spawn_ragit_server()
        mk_and_cd_tmp_dir()
        os.mkdir("base")
        os.chdir("base")

        # step 1: create a local knowledge-base
        #         base 1: a small base with 3 markdown files and an image
        cargo_run(["init"])
        cargo_run(["config", "--set", "model", "dummy"])
        write_string("sample1.txt", "Will AIs replace me?")
        write_string("sample2.txt", "I don't think so...")
        write_string("sample3.md", "![sample.png](sample.png)")
        shutil.copyfile("../../tests/images/empty.png", "sample.png")
        cargo_run(["add", "sample1.txt", "sample2.txt", "sample3.md"])
        cargo_run(["build"])
        cargo_run(["check"])

        # step 2: push the local knowledge-base to the server
        create_user(id="test-user", password="password")
        api_key = get_api_key(id="test-user", password="password")
        create_repo(user="test-user", repo="repo1", api_key=api_key)
        cargo_run(["push", "--remote=http://127.0.0.1:41127/test-user/repo1"])

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
        create_repo(user="test-user", repo="repo2", api_key=api_key)
        cargo_run(["push", "--remote=http://127.0.0.1:41127/test-user/repo2"])

        # step 5: clone samples from ragit.baehyunsol.com and push it to the local server
        os.chdir("..")

        for (base, url) in [
            ("git", "https://ragit.baehyunsol.com/sample/git"),
            ("ragit", "https://ragit.baehyunsol.com/sample/ragit"),
            ("rustc", "https://ragit.baehyunsol.com/sample/rustc"),
        ]:
            cargo_run(["clone", url], timeout=100)
            os.rename(base, base + "-cloned")
            os.chdir(base + "-cloned")
            cargo_run(["check"])
            get_repo_stat(user="test-user", repo=base, expected_status_code=404)
            create_repo(user="test-user", repo=base, api_key=api_key)
            assert get_repo_stat(user="test-user", repo=base) == (0, 0)  # (push, clone)
            cargo_run(["push", f"--remote=http://127.0.0.1:41127/test-user/{base}"])
            assert get_repo_stat(user="test-user", repo=base) == (1, 0)
            os.chdir("..")

        # step 6: clone and check base 1
        cargo_run(["clone", "http://127.0.0.1:41127/test-user/repo1"])
        os.chdir("repo1")
        cargo_run(["check"])
        assert "sample1.txt" not in cargo_run(["tfidf", "think"], stdout=True)
        assert "sample2.txt" in cargo_run(["tfidf", "think"], stdout=True)
        assert "sample1.txt" in cargo_run(["tfidf", "replace"], stdout=True)
        assert "sample2.txt" not in cargo_run(["tfidf", "replace"], stdout=True)

        # step 7: clone and check base 2
        os.chdir("..")
        cargo_run(["clone", "http://127.0.0.1:41127/test-user/repo2"])
        os.chdir("repo2")
        cargo_run(["check"])

        # step 8: clone and check cloned bases
        os.chdir("..")

        for (base, url) in [
            ("git", "http://127.0.0.1:41127/test-user/git"),
            ("ragit", "http://127.0.0.1:41127/test-user/ragit"),
            ("rustc", "http://127.0.0.1:41127/test-user/rustc"),
        ]:
            cargo_run(["clone", url])
            assert get_repo_stat(user="test-user", repo=base) == (1, 1)
            os.chdir(base)
            cargo_run(["check"])
            # `rag clone` builds an inverted-index
            assert cargo_run(["ii-status"], stdout=True).strip() == "complete"
            os.chdir("..")

    finally:
        if server_process is not None:
            server_process.kill()
