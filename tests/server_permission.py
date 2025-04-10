import os
from server import cargo_run, create_repo, create_user, health_check, request_json
import subprocess
import time
from utils import goto_root, mk_and_cd_tmp_dir

def server_permission():
    goto_root()
    os.chdir("crates/server")

    if health_check():
        raise Exception("ragit-server is already running. Please run this test in an isolated environment.")

    try:
        # step 0: run a ragit-server
        subprocess.Popen(["cargo", "run", "--release", "--", "truncate-all", "--force"]).wait()
        server_process = subprocess.Popen(["cargo", "run", "--release", "--features=log_sql", "--", "run", "--force-default-config"])
        os.chdir("../..")
        mk_and_cd_tmp_dir()

        # let's wait until `ragit-server` becomes healthy
        for _ in range(300):
            if health_check():
                break

            print("waiting for ragit-server to start...")
            time.sleep(1)

        else:
            raise Exception("failed to run `ragit-server`")

        create_user(name="test-user-1", email="sample1@email.com", password="12345678")
        create_user(name="test-user-2", email="sample2@email.com", password="abcdefgh")
        user_info1 = request_json(url="http://127.0.0.1:41127/user-list/test-user-1", raw_url=True)
        user_info2 = request_json(url="http://127.0.0.1:41127/user-list/test-user-2", raw_url=True)
        request_json(url="http://127.0.0.1:41127/user-list/test-user-3", raw_url=True, assert404=True)

        assert user_info1["email"] == "sample1@email.com"
        assert user_info2["email"] == "sample2@email.com"

        create_repo(user="test-user-1", repo="repo1", readme="hello, world")
        repo_info1 = request_json(url="http://127.0.0.1:41127/repo-list/test-user-1/repo1", raw_url=True)
        request_json(url="http://127.0.0.1:41127/repo-list/test-user-1/repo2", raw_url=True, assert404=True)

        assert repo_info1["name"] == "repo1"
        assert repo_info1["readme"] == "hello, world"
        assert repo_info1["pushed_at"] is None
        assert repo_info1["repo_size"] == 0

        cargo_run(["clone", "http://ragit.baehyunsol.com/sample/rustc"])
        os.chdir("rustc")
        cargo_run(["push", "--remote=http://127.0.0.1:41127/test-user-1/repo1"])
        os.chdir("..")

        repo_info2 = request_json(url="http://127.0.0.1:41127/repo-list/test-user-1/repo1", raw_url=True)

        assert repo_info2["pushed_at"] is not None
        assert repo_info2["repo_size"] > 0
        assert repo_info2["readme"] == "hello, world"

        # TODO: many more tests
        # TODO: create repository with different permissions, and send requests to the repository with/without api key

    finally:
        server_process.kill()
