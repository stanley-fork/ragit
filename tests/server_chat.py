import json
import os
import requests
from server import (
    create_repo,
    create_user,
    get_api_key,
    health_check,
    spawn_ragit_server,
)
import subprocess
import time
from typing import Optional
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir

def server_chat(test_model: str):
    goto_root()

    try:
        server_process = spawn_ragit_server()
        mk_and_cd_tmp_dir()

        # step 1: init sample 1 (rustc)
        cargo_run(["clone", "http://ragit.baehyunsol.com/sample/rustc", "sample1"])
        os.chdir("sample1")

        # There's a small quirk here:
        # `ragit-server` and local knowledge-base are looking at different `models.json` files.
        # The test runs on the server but `config --set model` alters the local knowledge-base.
        # It does so in order to get api key from the local knowledge-base.
        cargo_run(["config", "--set", "model", test_model])
        create_user(id="test-user", password="secure-password")
        server_api_key = get_api_key(id="test-user", password="secure-password")
        model_api_key = get_model_api_key()
        create_repo(user="test-user", repo="sample1", api_key=server_api_key)

        # set default model of server
        model_full_name = set_default_model(user="test-user", model_name=test_model, server_api_key=server_api_key)

        if model_api_key is not None:
            put_model_api_key(user="test-user", model_name=model_full_name, model_api_key=model_api_key, server_api_key=server_api_key)

        cargo_run(["push", "--configs", "--remote=http://127.0.0.1/test-user/sample1"])
        os.chdir("..")

        # step 2: init sample 2 (empty knowledge-base)
        os.mkdir("sample2")
        os.chdir("sample2")
        cargo_run(["init"])
        cargo_run(["config", "--set", "model", test_model])
        create_repo(user="test-user", repo="sample2", api_key=server_api_key)
        cargo_run(["push", "--configs", "--remote=http://127.0.0.1/test-user/sample2"])
        os.chdir("..")

        # step 3: let's ask questions!
        chat_id1 = requests.post("http://127.0.0.1:41127/test-user/sample1/chat-list").text
        chat_id2 = requests.post("http://127.0.0.1:41127/test-user/sample2/chat-list").text
        responses1 = []
        responses2 = []

        chat_list = requests.get("http://127.0.0.1:41127/test-user/sample1/chat-list").json()
        assert len(chat_list) == 1
        assert str(chat_list[0]["id"]) == chat_id1

        # TODO: what's the difference between multipart/form and body/form? I'm noob to this...
        responses1.append(requests.post(f"http://127.0.0.1:41127/test-user/sample1/chat/{chat_id1}", files={"query": "How does the rust compiler implement type system?"}).json())
        responses2.append(requests.post(f"http://127.0.0.1:41127/test-user/sample2/chat/{chat_id2}", files={"query": "How does the rust compiler implement type system?"}).json())

        responses1.append(requests.post(f"http://127.0.0.1:41127/test-user/sample1/chat/{chat_id1}", data={"query": "What do you mean by MIR?"}).json())
        responses2.append(requests.post(f"http://127.0.0.1:41127/test-user/sample2/chat/{chat_id2}", data={"query": "What do you mean by MIR?"}).json())

        responses1.append(requests.post(f"http://127.0.0.1:41127/test-user/sample1/chat/{chat_id1}", files={"query": "Thanks!"}).json())
        responses2.append(requests.post(f"http://127.0.0.1:41127/test-user/sample2/chat/{chat_id2}", files={"query": "Thanks!"}).json())

        history1 = requests.get(f"http://127.0.0.1:41127/test-user/sample1/chat/{chat_id1}").json()["history"]
        history2 = requests.get(f"http://127.0.0.1:41127/test-user/sample2/chat/{chat_id2}").json()["history"]

        chat_list = requests.get("http://127.0.0.1:41127/test-user/sample1/chat-list").json()
        assert len(chat_list) == 1
        assert str(chat_list[0]["id"]) == chat_id1

        for response in responses2:
            assert len(response["chunk_uids"]) == 0

        assert [(h["response"], h["multi_turn_schema"]) for h in history1] == [(r["response"], r["multi_turn_schema"]) for r in responses1]
        assert [(h["response"], h["multi_turn_schema"]) for h in history2] == [(r["response"], r["multi_turn_schema"]) for r in responses2]

        # build-search-index is idempotent
        for _ in range(3):
            assert requests.post("http://127.0.0.1:41127/test-user/sample1/build-search-index").status_code == 200
            assert requests.post("http://127.0.0.1:41127/test-user/sample2/build-search-index").status_code == 200

    finally:
        server_process.kill()

# It assumes that `rag config --set model _` is already run.
def get_model_api_key() -> Optional[str]:
    # `rag ls-models` do not dump these models
    if cargo_run(["config", "--get", "model"], stdout=True).strip() in ["dummy", "stdin", "error"]:
        return None

    with open(os.path.join(".ragit", "models.json"), "r") as f:
        models = json.load(f)

    model_full_name = json.loads(cargo_run(["ls-models", "--json", "--selected"], stdout=True).strip())[0]["name"]
    model = [model for model in models if model["name"] == model_full_name][0]

    if (api_key := model.get("api_key")) is not None:
        return api_key

    elif (api_env_var := model.get("api_env_var")) is not None:
        if (api_key := os.environ.get(api_env_var)) is not None:
            return api_key

        else:
            raise Exception(f"API key is not set. Please set the {api_env_var} environment variable.")

    # some models may not require an API key
    else:
        return None

# There's a small quirk here:
# ragit-server and local knowledge-base are looking at different `models.json` files.
# The same model might have (slightly) different names.
# What's even worse is that `model_name` is given by the test runner, who doesn't know about
# `models.json` at all. So it first tries to guess what the actual name of the model is.
# It returns the full name of the model.
def set_default_model(user: str, model_name: str, server_api_key: str) -> str:
    models = requests.get(
        f"http://127.0.0.1:41127/user-list/{user}/ai-model-list",
        headers={ "x-api-key": server_api_key },
    ).json()
    models = [model for model in models if model_name in model["name"] or model_name in model["api_name"]]

    if len(models) != 1:
        raise Exception(f"Model name {model_name} is ambiguous.")

    model_name = models[0]["name"]
    response = requests.put(
        f"http://127.0.0.1:41127/user-list/{user}/ai-model-list",
        json={ "default_model": model_name },
        headers={ "x-api-key": server_api_key },
    )
    assert response.status_code == 200, f"Failed to set default model: {response.text}"
    return model_name

def put_model_api_key(user: str, model_name: str, model_api_key: str, server_api_key: str):
    response = requests.put(
        f"http://127.0.0.1:41127/user-list/{user}/ai-model-list",
        json={ "model": model_name, "api_key": model_api_key },
        headers={ "x-api-key": server_api_key },
    )
    assert response.status_code == 200, f"Failed to set API key: {response.text}"
