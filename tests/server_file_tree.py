import os
from server import (
    create_repo,
    create_user,
    get_api_key,
    get_json,
    spawn_ragit_server,
)
import shutil
from typing import Optional
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
)

def server_file_tree():
    goto_root()
    server_process = None

    try:
        server_process = spawn_ragit_server()
        mk_and_cd_tmp_dir()
        shutil.copytree("../src", "./src")
        shutil.copytree("../RelNotes", "./RelNotes")
        shutil.copytree("../prompts", "./prompts")
        file_tree = create_file_tree()

        cargo_run(["init"])
        cargo_run(["config", "--set", "model", "dummy"])
        cargo_run(["add", "src", "RelNotes", "prompts"])
        cargo_run(["build"])
        cargo_run(["check"])

        create_user(id="test-user", password="12345678")
        api_key = get_api_key(id="test-user", password="12345678")
        create_repo(user="test-user", repo="ragit", api_key=api_key)
        cargo_run(["push", "--remote=http://127.0.0.1:41127/test-user/ragit"])

        run_test(file_tree)

        get_json(
            url="file-content",
            query={ "path": "invalid-file-name" },
            repo="ragit",
            expected_status_code=404,
        )
        c = get_json(
            url="file-content",
            query={ "path": "prompts/rerank_summary.pdl" },
            repo="ragit",
        )

        # This is another bug, but I'll just keep it here because I found this bug
        # with this test. This bug must be introduced at 42d170f48. The string
        # "<|schema|>" in a file is converted to "&lt;|schema|&gt;" before it's fed
        # to the pdl engine (escape), then later converted back to "<|schema|>" when
        # it's saved to a chunk. Well, it's supposed to.
        # But "&lt;|schema|&gt;" is found in a chunk. That means unescape logic is not applied.
        #
        # And of course I've added this case to `tests/pdl_escape.py`.
        assert "<|schema|>" in c["content"][0]["content"]

    finally:
        if server_process is not None:
            server_process.kill()

def create_file_tree():
    r = {}

    for d in os.listdir():
        if os.path.isdir(d):
            os.chdir(d)
            r[d] = create_file_tree()
            os.chdir("..")

        else:
            r[d] = ""

    return r

def run_test(file_tree: dict, prefix: Optional[list[str]] = None):
    prefix = prefix or []

    for (file, content) in file_tree.items():
        path = "/".join(prefix + [file])
        c = get_json(
            url="file-content",
            query={ "path": path },
            repo="ragit",
        )

        if isinstance(content, dict):
            children = [path + "/" + k + ("/" if isinstance(v, dict) else "") for k, v in content.items()]
            assert c["path"] == path + "/"
            assert c["type"] == "Directory"
            assert set(children) == set([c["path"] for c in c["children"]])
            run_test(content, prefix + [file])

            # If `foo/bar/` is a directory, `foo/bar` and `foo/bar/` both must work
            path2 = "/".join(prefix + [file + "/"])
            c2 = get_json(
                url="file-content",
                query={ "path": path },
                repo="ragit",
            )
            assert c == c2

        else:
            assert c["path"] == path
            assert c["type"] == "File"
            assert len(c["children"] or []) == 0
