import json
from migrate import checkout
import os
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir

# Ragit 0.3.3 deprecated config `max_titles` and 0.4.0 rejects getting/setting deprecated configs.
# Ragit 0.3.5 introduced a new config: `super_rerank`. I want to see if it's compatible with older versions.
def config():
    goto_root()
    mk_and_cd_tmp_dir()

    checkout("0.3.2")
    os.mkdir("0.3.2")
    os.chdir("0.3.2")
    cargo_run(["init"])

    os.chdir("..")
    checkout("0.3.5")
    os.mkdir("0.3.5")
    os.chdir("0.3.5")
    cargo_run(["init"])

    # 0.3.5 can read/write older versions' config.
    os.chdir("../0.3.2")
    assert cargo_run(["config", "--get", "super_rerank"], stdout=True).strip() == "false"
    assert "super_rerank" in cargo_run(["config", "--get-all"], stdout=True)
    cargo_run(["config", "--set", "super_rerank", "true"])
    assert cargo_run(["config", "--get", "super_rerank"], stdout=True).strip() == "true"

    # 0.3.2 doesn't know about `super_rerank`, but has no problem reading other configs.
    checkout("0.3.2")
    os.chdir("../0.3.5")
    cargo_run(["config", "--get", "max_summaries"])

    # 0.3.2 can set this value, but there's no effect.
    # assert cargo_run(["config", "--set", "super_rerank", "true"], check=False) != 0

    assert "max_titles" in cargo_run(["config", "--get-all"], stdout=True)
    cargo_run(["check"])

    # 0.4.0 can read old configs without any problem.
    # It acts as if there's no `max_titles` config at all!
    checkout("0.4.0")
    assert "max_titles" not in cargo_run(["config", "--get-all"], stdout=True)
    assert cargo_run(["config", "--get", "max_titles"], check=False) != 0
    assert "deprecated" in cargo_run(["config", "--get", "max_titles"], check=False, stderr=True)
    assert cargo_run(["config", "--set", "max_titles", "20"], check=False) != 0
    assert "deprecated" in cargo_run(["config", "--set", "max_titles", "20"], check=False, stderr=True)

    # extra check: 0.4.0 rejects setting invalid model names!
    cargo_run(["config", "--set", "model", "dummy"])
    models = json.loads(cargo_run(["ls-models", "--name-only", "--json"], stdout=True))

    # multiple models & no models
    assert len([m for m in models if "gpt" in m]) > 1
    assert len([m for m in models if "invalid-model-name" in m]) == 0

    # if the model name is ambiguous or invalid, it doesn't change the config
    assert cargo_run(["config", "--set", "model", "gpt"], check=False) != 0
    assert "dummy" in cargo_run(["config", "--get", "model"], stdout=True)
    assert cargo_run(["config", "--set", "model", "invalid-model-name"], check=False) != 0
    assert "dummy" in cargo_run(["config", "--get", "model"], stdout=True)
