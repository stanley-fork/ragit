import json
import os
from utils import (
    cargo_run,
    count_files,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

def outside():
    goto_root()
    mk_and_cd_tmp_dir()

    # this file will make ragit more confused
    write_string("sample.md", "Don't touch this!")

    os.mkdir("base")
    os.chdir("base")
    write_string("sample.md", "Hello, World!")
    cargo_run(["init"])

    # cannot add `../sample.md`
    assert cargo_run(["add", "../sample.md"], check=False) != 0
    assert "outside" in cargo_run(["add", "../sample.md"], check=False, stderr=True)

    # there used to be a bug: it's not supposed to be able to add `../../src/main.rs`, but it was.
    # I fixed it.
    assert cargo_run(["add", "../../src/main.rs"], check=False) != 0
    assert "outside" in cargo_run(["add", "../../src/main.rs"], check=False, stderr=True)

    cargo_run(["add", "../base/sample.md"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["build"])
    cargo_run(["check"])

    assert cargo_run(["cat-file", "../sample.md"], check=False) != 0
    assert cargo_run(["cat-file", "sample.md"], stdout=True).strip() == "Hello, World!"
    assert cargo_run(["cat-file", "../base/sample.md"], stdout=True).strip() == "Hello, World!"
    assert cargo_run(["ls-chunks", "../sample.md"], check=False) != 0
    assert len(chunk_uids := json.loads(cargo_run(["ls-chunks", "sample.md", "--uid-only", "--json"], stdout=True))) == 1
    assert len(json.loads(cargo_run(["ls-chunks", "../base/sample.md", "--json"], stdout=True))) == 1

    assert cargo_run(["rm", "../sample.md"], check=False) != 0
    cargo_run(["rm", "../base/sample.md"])
    assert count_files() == (0, 0, 0)

    # it's supposed to the same as `rag add .`
    cargo_run(["add", "../base"])
    assert count_files() == (1, 1, 0)  # (total, staged, processed)

    cargo_run(["build"])
    cargo_run(["check"])
    assert json.loads(cargo_run(["ls-chunks", "--uid-only", "--json"], stdout=True)) == chunk_uids
