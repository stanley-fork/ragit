import json
import os
from server import create_repo, create_user, health_check
import subprocess
import time
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, write_string

def clone_empty():
    goto_root()

    if health_check():
        raise Exception("ragit-server is already running. Please run this test in an isolated environment.")

    os.chdir("crates/server")

    try:
        # step 0: run a ragit-server
        subprocess.Popen(["cargo", "run", "--release", "--", "truncate-all", "--force"]).wait()
        server_process = subprocess.Popen(["cargo", "run", "--release", "--features=log_sql", "--", "run", "--force-default-config"])

        # before we push this to server, let's wait until `ragit-server` becomes healthy
        for _ in range(300):
            if health_check():
                break

            print("waiting for ragit-server to start...")
            time.sleep(1)

        else:
            raise Exception("failed to run `ragit-server`")

        os.chdir("../..")
        mk_and_cd_tmp_dir()
        create_user(name="test-user")
        create_repo(user="test-user", repo="empty-repo")

        # TODO: test it with different permissions
        #       e.g. cloning an empty repository without a permission
        result = cargo_run(["clone", "http://127.0.0.1:41127/test-user/empty-repo"], stderr=True)
        assert "empty" in result  # warning message: "you have cloned an empty knowledge-base"
        os.chdir("empty-repo")
        cargo_run(["check"])
        cargo_run(["config", "--set", "model", "dummy"])
        write_string("sample.txt", "Hello, World!")
        cargo_run(["add", "sample.txt"])
        cargo_run(["build"])
        cargo_run(["push"])
        chunk_list1 = set(json.loads(cargo_run(["ls-chunks", "--uid-only", "--json"], stdout=True)))
        cargo_run(["pull"])
        cargo_run(["check"])
        chunk_list2 = set(json.loads(cargo_run(["ls-chunks", "--uid-only", "--json"], stdout=True)))
        assert chunk_list1 == chunk_list2

        os.chdir("..")
        cargo_run(["clone", "http://127.0.0.1:41127/test-user/empty-repo", "not-empty-repo"])
        os.chdir("not-empty-repo")
        cargo_run(["check"])
        chunk_list3 = set(json.loads(cargo_run(["ls-chunks", "--uid-only", "--json"], stdout=True)))
        assert chunk_list2 == chunk_list3

    finally:
        server_process.kill()
