import json
import os
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
)

def pull_ragithub():
    goto_root()
    mk_and_cd_tmp_dir()
    cargo_run(["clone", "https://ragit.baehyunsol.com/sample/rustc"])
    os.chdir("rustc")
    uid1 = cargo_run(["uid"], stdout=True).strip()
    any_file = json.loads(cargo_run(["ls-files", "--name-only", "--json"], stdout=True))[0]
    cargo_run(["rm", any_file])
    uid2 = cargo_run(["uid"], stdout=True).strip()

    assert uid1 != uid2

    cargo_run(["pull"])
    uid3 = cargo_run(["uid"], stdout=True).strip()

    assert uid1 == uid3
