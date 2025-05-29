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

'''
create repo A
read uid of A: u1
push A
push A -> check if it says "Everything up-to-date"
edit A
read uid of A: u2
push A -> check if it's successfully pushed

clone A to repo B
read uid of B: u3
pull B -> check if it says "Already up to date"
read uid of B: u4
edit A
read uid of A: u5
push A -> check if it's successfully pushed
pull B -> check if it's successfully pulled
read uid of B: u6

assert u1 != u2
assert u2 == u3
assert u3 == u4
assert u4 != u5
assert u5 == u6
'''
def pull():
    goto_root()
    server_process = None

    try:
        server_process = spawn_ragit_server()

        create_user(id="test-user", password="password")
        api_key = get_api_key(id="test-user", password="password")
        create_repo(user="test-user", repo="test-repo", api_key=api_key)

        mk_and_cd_tmp_dir()

        # create repo A
        os.mkdir("A")
        os.chdir("A")
        cargo_run(["init"])
        shutil.copyfile("../../tests/images/empty.png", "sample.png")
        write_string("sample1.md", "Hi! this is an image: ![](sample.png)")
        cargo_run(["add", "sample1.md"])
        cargo_run(["config", "--set", "model", "dummy"])
        cargo_run(["build"])

        # read uid of A: u1
        u1 = cargo_run(["uid"], stdout=True).strip()

        # push A
        cargo_run(["push", "--remote=http://127.0.0.1/test-user/test-repo"])

        # push A -> check if it says "Everything up-to-date"
        assert "Everything up-to-date" in cargo_run(["push", "--remote=http://127.0.0.1/test-user/test-repo"], stdout=True)

        # edit A
        cargo_run(["meta", "--set", "key", "value"])

        # read uid of A: u2
        u2 = cargo_run(["uid"], stdout=True).strip()

        # push A -> check if it's successfully pushed
        assert "Everything up-to-date" not in cargo_run(["push", "--remote=http://127.0.0.1/test-user/test-repo"], stdout=True)

        # clone A to repo B
        os.chdir("..")
        cargo_run(["clone", "http://127.0.0.1/test-user/test-repo", "B"])
        os.chdir("B")

        # read uid of B: u3
        u3 = cargo_run(["uid"], stdout=True).strip()

        # pull B -> check if it says "Already up to date"
        assert "Already up to date" in cargo_run(["pull"], stdout=True).strip()

        # read uid of B: u4
        u4 = cargo_run(["uid"], stdout=True).strip()

        # edit A
        os.chdir("../A")
        shutil.copyfile("../../tests/images/empty.jpg", "sample.jpg")
        write_string("sample2.md", "Hi! this is an image: ![](sample.jpg)")
        cargo_run(["add", "sample2.md"])
        cargo_run(["build"])

        # read uid of A: u5
        u5 = cargo_run(["uid"], stdout=True).strip()

        # push A -> check if it's successfully pushed
        assert "Everything up-to-date" not in cargo_run(["push", "--remote=http://127.0.0.1/test-user/test-repo"], stdout=True)

        # pull B -> check if it's successfully pulled
        os.chdir("../B")
        assert "Already up to date" not in cargo_run(["pull"], stdout=True).strip()

        # read uid of B: u6
        u6 = cargo_run(["uid"], stdout=True).strip()

        assert u1 != u2
        assert u2 == u3
        assert u3 == u4
        assert u4 != u5
        assert u5 == u6

    finally:
        if server_process is not None:
            server_process.kill()
