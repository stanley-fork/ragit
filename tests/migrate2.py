from migrate import checkout
import os
import shutil
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir

def migrate2():
    goto_root()
    mk_and_cd_tmp_dir()
    curr_version = "0.3.2"

    # 0.1.1's clone is not compatible with the current version of ragit-server
    for old_version in ["0.2.0", "0.2.1", "0.3.0", "0.3.1"]:
        checkout(old_version)
        cargo_run(["clone", "http://ragit.baehyunsol.com/sample/ragit"])
        cargo_run(["clone", "http://ragit.baehyunsol.com/sample/git"])
        cargo_run(["clone", "http://ragit.baehyunsol.com/sample/rustc"])
        checkout(curr_version)

        for base in ["ragit", "git", "rustc"]:
            os.chdir(base)
            cargo_run(["migrate"])
            cargo_run(["check"])
            os.chdir("..")
            shutil.rmtree(base)
