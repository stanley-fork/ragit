import os
from utils import (
    cargo_run,
    count_files,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

# it first looks for `.ragignore`, then `.gitignore`
# you can never `rag add .git` or `rag add .ragit`, even with `--force`
def ignore():
    goto_root()
    mk_and_cd_tmp_dir()
    write_string(".ragignore", "file1\ndir1/")
    write_string(".gitignore", "file2\ndir2/")
    cargo_run(["init"])

    write_string("file1", "")
    write_string("file2", "")
    os.mkdir("dir1")
    os.mkdir("dir2")
    write_string("dir1/file3", "")
    write_string("dir2/file4", "")

    cargo_run(["add", "--all"])
    assert ".gitignore" in cargo_run(["ls-files"], stdout=True)
    assert "file2" in cargo_run(["ls-files"], stdout=True)
    assert "dir2/file4" in cargo_run(["ls-files"], stdout=True)
    assert count_files() == (3, 3, 0)

    cargo_run(["add", "--force", "dir1"])
    assert "dir1/file3" in cargo_run(["ls-files"], stdout=True)
    assert count_files() == (4, 4, 0)
    cargo_run(["rm", "-r", "dir1"])
    cargo_run(["add", "--force", "dir1/file3"])
    assert "dir1/file3" in cargo_run(["ls-files"], stdout=True)
    assert count_files() == (4, 4, 0)

    cargo_run(["add", "--force", "--all"])
    assert count_files() == (5, 5, 0)
    assert "file1" in cargo_run(["ls-files"], stdout=True)
    assert "dir1/file3" in cargo_run(["ls-files"], stdout=True)
    assert ".ragit/" not in cargo_run(["ls-files"], stdout=True)
    assert ".git/" not in cargo_run(["ls-files"], stdout=True)

    cargo_run(["rm", "--all"])
    os.remove(".ragignore")
    cargo_run(["add", "--all"])
    assert ".gitignore" in cargo_run(["ls-files"], stdout=True)
    assert "file1" in cargo_run(["ls-files"], stdout=True)
    assert "dir1/file3" in cargo_run(["ls-files"], stdout=True)
    assert count_files() == (3, 3, 0)
