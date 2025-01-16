import os
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir

def clone2():
    goto_root()
    mk_and_cd_tmp_dir()

    for repo_name, url in [
        ("git", "http://ragit.baehyunsol.com/sample/git"),
        ("ragit", "http://ragit.baehyunsol.com/sample/ragit"),
        ("rustc", "http://ragit.baehyunsol.com/sample/rustc"),
    ]:
        cargo_run(["clone", url], timeout=300.0)
        os.chdir(repo_name)
        cargo_run(["check"])
