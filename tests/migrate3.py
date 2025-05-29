import json
from migrate import checkout
import os
from server import (
    create_repo,
    create_user,
    get_api_key,
    spawn_ragit_server,
)
import shutil
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

def migrate3():
    goto_root()
    server_process = None

    try:
        server_process = spawn_ragit_server()
        versions = ["0.3.0", "0.3.5", "0.4.0"]
        chunks = {}

        mk_and_cd_tmp_dir()
        create_user(id="test-user", password="password")
        api_key = get_api_key(id="test-user", password="password")

        for version in versions:
            create_repo(user="test-user", repo=f"base-{version}", api_key=api_key)

        # step 1: It creates different knowledge-bases with different versions.
        #         It pushes each knowledge-base to the server.
        for version in versions:
            checkout(version)
            os.mkdir(f"base-{version}")
            os.chdir(f"base-{version}")
            cargo_run(["init"])
            cargo_run(["config", "--set", "model", "dummy"])
            write_string("sample1.txt", "Will AIs replace me?")
            write_string("sample2.txt", "AI가 개발자를 대체하게 될까요?")
            cargo_run(["add", "sample1.txt", "sample2.txt"])
            cargo_run(["build"])
            cargo_run(["check"])
            chunks[version] = set(json.loads(cargo_run(["ls-chunks", "--json", "--uid-only"], stdout=True)))
            cargo_run(["push", f"--remote=http://127.0.0.1/test-user/base-{version}"])
            os.chdir("..")
            shutil.rmtree(f"base-{version}")

        # step 2: Clone each knowledge-base with all the versions.
        for ragit_version in versions:
            checkout(ragit_version)

            for clone_version in versions:
                cargo_run(["clone", f"http://127.0.0.1/test-user/base-{clone_version}"])
                os.chdir(f"base-{clone_version}")
                cargo_run(["check"])
                assert chunks[clone_version] == set(json.loads(cargo_run(["ls-chunks", "--json", "--uid-only"], stdout=True)))
                os.chdir("..")
                shutil.rmtree(f"base-{clone_version}")

        # An extra step
        # Come to think about it, a chunk uid must be the same regardless of ragit versions, right?
        assert all([c == chunks[versions[0]] for c in chunks.values()])

    finally:
        if server_process is not None:
            server_process.kill()
