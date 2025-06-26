import json
import re
from server import (
    create_user,
    get_api_key,
    put_json,
    spawn_ragit_server,
)
from typing import Optional, Tuple
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir

def fetch_models():
    goto_root()
    server_process = None

    try:
        server_process = spawn_ragit_server()
        create_user(id="test-user", password="12345678")
        admin_api_key = get_api_key(id="test-user", password="12345678")

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

        # upload a new model and fetch the model
        new_model = {
            "name": "test-model-1234",
            "api_name": "who-cares-1234",
            "api_provider": "openai",
            "api_url": None,
            "can_read_images": False,
            "input_price": 0.0,
            "output_price": 0.0,
            "explanation": None,
            "api_env_var": "OPENAI_API_KEY",
            "tags": [],
        }
        put_json(
            url="http://127.0.0.1:41127/ai-model-list",
            body=new_model,
            raw_url=True,
            api_key=admin_api_key,
        )

        assert count_fetch_models(new_model["name"], existing_only=True) == (0, 0)
        assert count_fetch_models(new_model["name"]) == (1, 0)
        assert count_fetch_models(new_model["name"]) == (0, 0)

        local_models = ls_models()
        new_model_local = [model for model in local_models if model["name"] == new_model["name"]][0]
        assert new_model["api_provider"] == new_model_local["api_provider"]

        # update the new model and fetch the model
        new_model["api_provider"] = "google"
        put_json(
            url="http://127.0.0.1:41127/ai-model-list",
            body=new_model,
            raw_url=True,
            api_key=admin_api_key,
        )

        assert count_fetch_models(new_model["name"]) == (0, 1)
        assert count_fetch_models(new_model["name"]) == (0, 0)

        local_models = ls_models()
        new_model_local = [model for model in local_models if model["name"] == new_model["name"]][0]
        assert new_model["api_provider"] == new_model_local["api_provider"]

        # let's do another test! before that, we have to reset the local models
        cargo_run(["model", "--remove", "--all"])
        assert len(ls_models()) == 0

        assert count_fetch_models(name=None, existing_only=True) == (0, 0)
        fetched, updated = count_fetch_models(name=None)
        assert fetched > 0 and updated == 0

        local_models = ls_models()
        new_model_local = [model for model in local_models if model["name"] == new_model["name"]][0]
        assert new_model["api_provider"] == new_model_local["api_provider"]

        new_model["api_provider"] = "openai"
        put_json(
            url="http://127.0.0.1:41127/ai-model-list",
            body=new_model,
            raw_url=True,
            api_key=admin_api_key,
        )

        assert count_fetch_models(name=None, existing_only=True) == (0, 1)

        local_models = ls_models()
        new_model_local = [model for model in local_models if model["name"] == new_model["name"]][0]
        assert new_model["api_provider"] == new_model_local["api_provider"]

        cargo_run(["model", "--remove", new_model["name"]])

        new_model["api_provider"] = "google"
        put_json(
            url="http://127.0.0.1:41127/ai-model-list",
            body=new_model,
            raw_url=True,
            api_key=admin_api_key,
        )

        assert count_fetch_models(name=None, existing_only=True) == (0, 0)
        assert count_fetch_models(name=None, existing_only=False) == (1, 0)

        local_models = ls_models()
        new_model_local = [model for model in local_models if model["name"] == new_model["name"]][0]
        assert new_model["api_provider"] == new_model_local["api_provider"]

        # test the default remote (https://ragit.baehyunsol.com)
        default_remote_models = json.loads(cargo_run(["model", "--search", "gpt", "--json"], stdout=True))
        assert len(default_remote_models) > 0

    finally:
        if server_process is not None:
            server_process.kill()

def ls_models():
    return json.loads(cargo_run(["ls-models", "--json"], stdout=True))

def count_fetch_models(
    name: Optional[str],
    existing_only: bool = False,
    remote: str = "http://127.0.0.1:41127",
) -> Tuple[int, int]:  # (fetched, updated)
    existing_only = ["--existing-only"] if existing_only else []
    name = [name] if name is not None else ["--all"]
    s = cargo_run(["model", "--fetch", *name, *existing_only, f"--remote={remote}"], stdout=True)

    # maybe I should implement `--json` option for `rag model --fetch`
    r = re.search(r"fetched\s(\d+)\snew\smodels.+updated\s(\d+)\smodels", s)
    fetched, updated = r.groups()
    return (int(fetched), int(updated))
