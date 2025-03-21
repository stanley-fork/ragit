from migrate import checkout
import os
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir

# Ragit 0.3.5 introduced a new config: `super_rerank`. I want to see if it's compatible with older versions.
def config():
    goto_root()
    mk_and_cd_tmp_dir()

    checkout("0.3.3")
    os.mkdir("0.3.3")
    os.chdir("0.3.3")
    cargo_run(["init"])

    os.chdir("..")
    checkout("0.3.5")
    os.mkdir("0.3.5")
    os.chdir("0.3.5")
    cargo_run(["init"])

    # 0.3.5 can read/write older versions' config.
    os.chdir("../0.3.3")
    assert cargo_run(["config", "--get", "super_rerank"], stdout=True).strip() == "false"
    assert "super_rerank" in cargo_run(["config", "--get-all"], stdout=True)
    cargo_run(["config", "--set", "super_rerank", "true"])
    assert cargo_run(["config", "--get", "super_rerank"], stdout=True).strip() == "true"

    # 0.3.3 doesn't know about `super_rerank`, but has no problem reading other configs.
    checkout("0.3.3")
    os.chdir("../0.3.5")
    cargo_run(["config", "--get", "max_summaries"])
    assert cargo_run(["config", "--set", "super_rerank", "true"], check=False) != 0
    cargo_run(["config", "--get-all"])
    cargo_run(["check"])
