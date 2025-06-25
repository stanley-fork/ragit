import json
import re
from server import spawn_ragit_server
from typing import Tuple
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir

def fetch_models():
    goto_root()
    server_process = None

    try:
        server_process = spawn_ragit_server()
        mk_and_cd_tmp_dir()
        cargo_run(["init"])
        models = ls_models()

        for model in models:
            cargo_run(["model", "--remove", model["name"]])

        assert len(ls_models()) == 0

        server_models = json.loads(cargo_run(["model", "--search", "gpt", "--json", "--remote=http://127.0.0.1:41127"], stdout=True))
        assert len(server_models) > 0

        for model in server_models:
            assert count_fetch_models(model["name"], existing_only=True) == (0, 0)  # (fetched, updated)
            assert count_fetch_models(model["name"]) == (1, 0)
            assert count_fetch_models(model["name"]) == (0, 0)

        local_models = ls_models()
        assert set([model["name"] for model in server_models]) == set([model["name"] for model in local_models])

        # TODO: update server's ai model, run `rag model --fetch` and check if the local model info is updated

        # test the default remote (https://ragit.baehyunsol.com)
        default_remote_models = json.loads(cargo_run(["model", "--search", "gpt", "--json"], stdout=True))
        assert len(default_remote_models) > 0

    finally:
        if server_process is not None:
            server_process.kill()

def ls_models():
    return json.loads(cargo_run(["ls-models", "--json"], stdout=True))

def count_fetch_models(
    name: str,
    existing_only: bool = False,
    remote: str = "http://127.0.0.1:41127",
) -> Tuple[int, int]:  # (fetched, updated)
    existing_only = ["--existing-only"] if existing_only else []
    s = cargo_run(["model", "--fetch", name, *existing_only, f"--remote={remote}"], stdout=True)

    # maybe I should implement `--json` option for `rag model --fetch`
    r = re.match(r"fetched\s(\d+)\snew\smodels.+updated\s(\d+)\smodels", s)
    fetched, updated = r.groups()
    return (int(fetched), int(updated))
