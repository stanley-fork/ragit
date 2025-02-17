import os
from utils import (
    cargo_run,
    count_chunks,
    count_files,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

def symlink():
    goto_root()
    mk_and_cd_tmp_dir()

    # base 1: 1 file and 1 symlink
    os.mkdir("base1")
    os.chdir("base1")
    write_string("x.py", "print('Hello, world!')")
    os.symlink("x.py", "y.py")
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["add", "."])
    cargo_run(["build"])

    assert count_chunks() == 1
    assert count_files() == (1, 0, 1)

    # base 2: simple cyclic symlink
    os.chdir("..")
    os.mkdir("base2")
    os.chdir("base2")
    os.symlink("x.py", "y.py")
    os.symlink("y.py", "x.py")
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["add", "."])
    cargo_run(["build"])

    # base 3: another cyclic symlink
    os.chdir("..")
    os.mkdir("base3")
    os.chdir("base3")

    # a/x.py
    # a/b/link -> a
    os.mkdir("a")
    os.chdir("a")
    write_string("x.py", "print('Hello, world!')")
    os.mkdir("b")
    os.chdir("b")
    os.symlink("../", "link")
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    os.chdir("../..")
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["add", "."])
    cargo_run(["build"])

    assert count_chunks() == 1
    assert count_files() == (1, 0, 1)
