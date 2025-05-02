import json
import os
from server import (
    create_repo,
    create_user,
    get_api_key,
    health_check,
    spawn_ragit_server,
)
import subprocess
import time
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, write_string

def clone_empty():
    goto_root()

    try:
        server_process = spawn_ragit_server()
        mk_and_cd_tmp_dir()
        create_user(id="test-user", password="87654321")
        api_key = get_api_key(id="test-user", password="87654321")
        create_repo(user="test-user", repo="empty-repo", api_key=api_key)

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
