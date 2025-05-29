import os
from server import (
    create_repo,
    create_user,
    get_api_key,
    get_json,
    health_check,
    post_json,
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
    write_string,
)

def server_permission():
    goto_root()
    server_process = None

    try:
        server_process = spawn_ragit_server()
        mk_and_cd_tmp_dir()

        create_user(id="admin", email="i.am@admin.com", password="strong-password")
        admin_api_key = get_api_key(id="admin", password="strong-password")
        create_user(id="test-user-1", email="sample1@email.com", password="12345678", api_key=admin_api_key)
        user1_api_key = get_api_key(id="test-user-1", password="12345678")
        create_user(id="test-user-2", email="sample2@email.com", password="abcdefgh", api_key=admin_api_key)
        user2_api_key = get_api_key(id="test-user-2", password="abcdefgh")

        user1_info = get_json(url="http://127.0.0.1:41127/user-list/test-user-1", raw_url=True)
        user2_info = get_json(url="http://127.0.0.1:41127/user-list/test-user-2", raw_url=True)
        get_json(url="http://127.0.0.1:41127/user-list/test-user-3", raw_url=True, expected_status_code=404)

        assert user1_info["email"] == "sample1@email.com"
        assert user2_info["email"] == "sample2@email.com"

        create_repo(user="test-user-1", repo="repo1", readme="hello, world", api_key=user1_api_key)
        repo_info1 = get_json(url="http://127.0.0.1:41127/repo-list/test-user-1/repo1", raw_url=True)
        get_json(url="http://127.0.0.1:41127/repo-list/test-user-1/repo2", raw_url=True, expected_status_code=404)

        assert repo_info1["name"] == "repo1"
        assert repo_info1["readme"] == "hello, world"
        assert repo_info1["pushed_at"] is None
        assert repo_info1["repo_size"] == 0

        cargo_run(["clone", "http://ragit.baehyunsol.com/sample/rustc"])
        os.chdir("rustc")
        cargo_run(["push", "--remote=http://127.0.0.1:41127/test-user-1/repo1"])
        os.chdir("..")

        repo_info2 = get_json(url="http://127.0.0.1:41127/repo-list/test-user-1/repo1", raw_url=True)

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
            create_repo(
                user="test-user-1",
                repo=rand_word(),
                api_key=None,
                expected_status_code=403,
            )

            # step 1: public-read vs private-read
            # step 1.1: correct api key
            get_json(
                url=f"http://127.0.0.1:41127/repo-list/test-user-1/{repo_name}",
                raw_url=True,
                api_key=user1_api_key,
                expected_status_code=200,
            )

            # step 1.2: wrong api key
            get_json(
                url=f"http://127.0.0.1:41127/repo-list/test-user-1/{repo_name}",
                raw_url=True,
                api_key=user2_api_key,
                expected_status_code=200 if public_read else 404,
            )
            get_json(
                url=f"http://127.0.0.1:41127/repo-list/test-user-1/{repo_name}",
                raw_url=True,
                api_key="an-api-key-that-doesnt-make-sense-at-all",
                expected_status_code=200 if public_read else 404,
            )

            # step 1.3: no api key
            get_json(
                url=f"http://127.0.0.1:41127/repo-list/test-user-1/{repo_name}",
                raw_url=True,
                api_key=None,
                expected_status_code=200 if public_read else 404,
            )

            # step 2: public-write vs private-write
            # step 2.1: correct api key
            post_json(
                url=f"http://127.0.0.1:41127/test-user-1/{repo_name}/build-search-index",
                raw_url=True,
                api_key=user1_api_key,
                body={},  # it doesn't require body
                expected_status_code=200,
            )

            # step 2.2: wrong api key
            post_json(
                url=f"http://127.0.0.1:41127/test-user-1/{repo_name}/build-search-index",
                raw_url=True,
                api_key=user2_api_key,
                body={},  # it doesn't require body
                expected_status_code=200 if public_write else 404,
            )
            post_json(
                url=f"http://127.0.0.1:41127/test-user-1/{repo_name}/build-search-index",
                raw_url=True,
                api_key="an-api-key-that-doesnt-make-sense-at-all",
                body={},  # it doesn't require body
                expected_status_code=200 if public_write else 404,
            )

            # step 2.3: no api key
            post_json(
                url=f"http://127.0.0.1:41127/test-user-1/{repo_name}/build-search-index",
                raw_url=True,
                api_key=None,
                body={},  # it doesn't require body
                expected_status_code=200 if public_write else 404,
            )

            # step 3: public-clone vs private-clone
            # step 3.1: correct api key
            os.environ["RAGIT_API_KEY"] = user1_api_key
            cargo_run(["clone", f"http://127.0.0.1:41127/test-user-1/{repo_name}"])
            shutil.rmtree(repo_name)

            # step 3.2: wrong api key
            os.environ["RAGIT_API_KEY"] = user2_api_key
            result = cargo_run(["clone", f"http://127.0.0.1:41127/test-user-1/{repo_name}"], check=False)

            if public_clone:
                shutil.rmtree(repo_name)
                assert result == 0

            else:
                assert result != 0

            # step 3.3: no api key
            del os.environ["RAGIT_API_KEY"]
            result = cargo_run(["clone", f"http://127.0.0.1:41127/test-user-1/{repo_name}"], check=False)

            if public_clone:
                shutil.rmtree(repo_name)
                assert result == 0

            else:
                assert result != 0

            # step 4: public-push vs private-push
            # step 4.1: correct api key
            os.environ["RAGIT_API_KEY"] = user1_api_key
            cargo_run(["clone", f"http://127.0.0.1:41127/test-user-1/{repo_name}"])
            os.chdir(repo_name)

            # It doesn't push anything if the local knowledge-base and remote knowledge-base
            # are the same. So we have to add a random modification.
            write_string(file_name := (rand_word() + ".md"), rand_word())
            cargo_run(["add", file_name])
            cargo_run(["config", "--set", "model", "dummy"])
            cargo_run(["build"])

            cargo_run(["push"])

            # step 4.2: wrong api key
            os.environ["RAGIT_API_KEY"] = user2_api_key

            write_string(file_name := (rand_word() + ".md"), rand_word())
            cargo_run(["add", file_name])
            cargo_run(["config", "--set", "model", "dummy"])
            cargo_run(["build"])

            result = cargo_run(["push"], check=False)

            if public_push:
                assert result == 0

            else:
                assert result != 0

            # step 4.3: no api key
            del os.environ["RAGIT_API_KEY"]

            write_string(file_name := (rand_word() + ".md"), rand_word())
            cargo_run(["add", file_name])
            cargo_run(["config", "--set", "model", "dummy"])
            cargo_run(["build"])

            result = cargo_run(["push"], check=False)

            if public_push:
                assert result == 0

            else:
                assert result != 0

            os.chdir("..")
            shutil.rmtree(repo_name)

            # TODO: public-chat vs private-chat

    finally:
        if server_process is not None:
            server_process.kill()
