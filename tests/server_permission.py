import os
from server import (
    create_repo,
    create_user,
    get_api_key,
    health_check,
    request_json,
    spawn_ragit_server,
)
import shutil
import subprocess
import time
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    rand_word,
)

def server_permission():
    goto_root()

    try:
        server_process = spawn_ragit_server()
        mk_and_cd_tmp_dir()

        create_user(id="admin", email="i.am@admin.com", password="strong-password")
        admin_api_key = get_api_key(id="admin", password="strong-password")
        create_user(id="test-user-1", email="sample1@email.com", password="12345678", api_key=admin_api_key)
        user1_api_key = get_api_key(id="test-user-1", password="12345678")
        create_user(id="test-user-2", email="sample2@email.com", password="abcdefgh", api_key=admin_api_key)
        user2_api_key = get_api_key(id="test-user-2", password="abcdefgh")

        user1_info = request_json(url="http://127.0.0.1:41127/user-list/test-user-1", raw_url=True)
        user2_info = request_json(url="http://127.0.0.1:41127/user-list/test-user-2", raw_url=True)
        request_json(url="http://127.0.0.1:41127/user-list/test-user-3", raw_url=True, expected_status_code=404)

        assert user1_info["email"] == "sample1@email.com"
        assert user2_info["email"] == "sample2@email.com"

        create_repo(user="test-user-1", repo="repo1", readme="hello, world", api_key=user1_api_key)
        repo_info1 = request_json(url="http://127.0.0.1:41127/repo-list/test-user-1/repo1", raw_url=True)
        request_json(url="http://127.0.0.1:41127/repo-list/test-user-1/repo2", raw_url=True, expected_status_code=404)

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

        for permission in [
            # (read, write, clone, push, chat),
            (False, True, True, True, True),
            (True, False, True, True, True),
            (True, True, False, True, True),
            (True, True, True, False, True),
            (True, True, True, True, False),
        ]:
            public_read, public_write, public_clone, public_push, public_chat = permission
            repo_name = rand_word(english_only=True)  # I'm too lazy to name the repo :)
            create_repo(
                user="test-user-1",
                repo=repo_name,
                api_key=user1_api_key,
                public_read=public_read,
                public_write=public_write,
                public_clone=public_clone,
                public_push=public_push,
                public_chat=public_chat,
            )

            # cannot create a repo with the same name
            create_repo(
                user="test-user-1",
                repo=repo_name,
                api_key=user1_api_key,
                expected_status_code=400,
            )

            # cannot create a repo with a wrong api key
            create_repo(
                user="test-user-1",
                repo=rand_word(),
                api_key=user2_api_key,
                expected_status_code=403,
            )

            # user 1 can always read, but user 2 can read only if it's public-read
            request_json(
                url=f"http://127.0.0.1:41127/repo-list/test-user-1/{repo_name}",
                raw_url=True,
                api_key=user1_api_key,
                expected_status_code=200,
            )
            request_json(
                url=f"http://127.0.0.1:41127/repo-list/test-user-1/{repo_name}",
                raw_url=True,
                api_key=user2_api_key,
                expected_status_code=200 if public_read else 404,
            )

            # TODO: public-write

            # user 1 can always clone, but user 2 can clone only if it's public-clone
            os.environ["RAGIT_API_KEY"] = user1_api_key
            cargo_run(["clone", f"http://127.0.0.1:41127/test-user-1/{repo_name}"])
            shutil.rmtree(repo_name)
            os.environ["RAGIT_API_KEY"] = user2_api_key
            result = cargo_run(["clone", f"http://127.0.0.1:41127/test-user-1/{repo_name}"], check=False)

            if public_clone:
                shutil.rmtree(repo_name)
                assert result == 0

            else:
                assert result != 0

            # user 1 can always push, but user 2 can push only if it's public-push
            os.environ["RAGIT_API_KEY"] = user1_api_key
            cargo_run(["clone", f"http://127.0.0.1:41127/test-user-1/{repo_name}"])
            os.chdir(repo_name)
            cargo_run(["push"])
            os.environ["RAGIT_API_KEY"] = user2_api_key
            result = cargo_run(["push"], check=False)

            if public_push:
                assert result == 0

            else:
                assert result != 0

            os.chdir("..")
            shutil.rmtree(repo_name)

            # TODO: public-chat

    finally:
        server_process.kill()
