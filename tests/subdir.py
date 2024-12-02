import os
from utils import (
    cargo_run,
    count_files,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

def subdir():
    goto_root()
    mk_and_cd_tmp_dir()
    write_string("1.txt", "hello!")
    write_string("2.txt", "hi!")
    write_string("3.txt", "hahaha")

    cargo_run(["init"])
    cargo_run(["add", "1.txt", "2.txt", "3.txt"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["build"])

    for _ in range(4):
        assert count_files() == (3, 0, 3)
        os.mkdir("sub")
        write_string("4.txt", "can you see me?")
        write_string("5.txt", "can you see me?")
        write_string("sub/6.txt", "can you see me?")
        write_string("sub/7.txt", "can you see me?")
        cargo_run(["add", "4.txt", "sub/6.txt"])
        cargo_run(["add", os.path.join(os.getcwd(), "5.txt")])
        cargo_run(["add", os.path.join(os.getcwd(), "sub/7.txt")])

        cargo_run(["build"])
        assert count_files() == (7, 0, 7)
        mk_and_cd_tmp_dir()
        cargo_run(["rm", "../4.txt", "../sub/6.txt"])
        cargo_run(["rm", os.path.join(os.getcwd(), "../5.txt")])
        cargo_run(["rm", os.path.join(os.getcwd(), "../sub/7.txt")])

    os.chdir("../../../../")
    os.chdir(".ragit")

    # NOTE: if I do the same thing in git, git dies with "fatal: this operation must be run in a work tree"
    #       I have to do something...
    assert count_files() == (3, 0, 3)

    # It's not supposed to add files in `.ragit/`.
    # If you run `git add .git/HEAD`, it terminates the process with 0, but does not add the file.
    cargo_run(["add", "index.json"])
    assert count_files() == (3, 0, 3)

    os.chdir("..")
    cargo_run(["add", ".ragit/index.json"])
    assert count_files() == (3, 0, 3)

    mk_and_cd_tmp_dir()
    cargo_run(["add", "../.ragit/index.json"])
    assert count_files() == (3, 0, 3)
