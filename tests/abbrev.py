import json
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

def abbrev():
    goto_root()
    mk_and_cd_tmp_dir()

    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    write_string("sample.md", "Hello, World!")
    cargo_run(["add", "sample.md"])
    cargo_run(["build"])

    long_kb_uid1 = cargo_run(["uid"], stdout=True).strip()
    long_kb_uid2 = cargo_run(["uid", "--abbrev=64"], stdout=True).strip()
    short_kb_uid1 = cargo_run(["uid", "--abbrev=9"], stdout=True).strip()
    assert long_kb_uid1 == long_kb_uid2
    assert len(long_kb_uid1) == 64
    assert len(short_kb_uid1) == 9
    assert long_kb_uid1.startswith(short_kb_uid1)

    long_chunk_uid = json.loads(cargo_run(["ls-chunks", "--uid-only", "--json", "--abbrev=64"], stdout=True))[0]
    short_chunk_uid = json.loads(cargo_run(["ls-chunks", "--uid-only", "--json", "--abbrev=7"], stdout=True))[0]
    assert long_chunk_uid.startswith(short_chunk_uid)

    # The default `--abbrev` must be between 7 and 64
    no_json = cargo_run(["ls-chunks"], stdout=True)
    assert short_chunk_uid in no_json
    assert long_chunk_uid not in no_json
