import os
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir

def logs(test_model: str):
    goto_root()
    mk_and_cd_tmp_dir()

    cargo_run(["init"])

    for test_model in [test_model, "dummy"]:
        cargo_run(["config", "--set", "model", test_model])
        cargo_run(["config", "--set", "dump_log", "true"])
        logs = [] if not os.path.exists(".ragit/logs") else os.listdir(".ragit/logs")
        assert len(logs) == 0

        cargo_run(["query", "why is the sky blue?"])
        logs = [] if not os.path.exists(".ragit/logs") else os.listdir(".ragit/logs")
        assert len(logs) > 0

        cargo_run(["gc", "--logs"])
        logs = [] if not os.path.exists(".ragit/logs") else os.listdir(".ragit/logs")
        assert len(logs) == 0

        cargo_run(["config", "--set", "dump_log", "false"])
        cargo_run(["query", "why is the sky blue?"])
        logs = [] if not os.path.exists(".ragit/logs") else os.listdir(".ragit/logs")
        assert len(logs) == 0
