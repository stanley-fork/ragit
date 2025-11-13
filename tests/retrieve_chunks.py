import json
import os
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, send_message

def retrieve_chunks(test_model: str):
    goto_root()
    mk_and_cd_tmp_dir()
    cargo_run(["clone", "https://ragit.baehyunsol.com/sample/ragit"])
    os.chdir("ragit")
    question = "how do I retrieve chunks in a ragit knowledge-base?"
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["config", "--set", "max_retrieval", "5"])

    # a dummy model doesn't fail, but doesn't retrieve anything
    chunks = cargo_run(["retrieve-chunks", "--model=dummy", question, "--json"], stdout=True)
    chunks = json.loads(chunks)
    assert chunks == []

    # test `--max-retrieval` option
    chunks = cargo_run(["retrieve-chunks", f"--model={test_model}", question, "--max-retrieval=1", "--json"], stdout=True)
    chunks = json.loads(chunks)
    assert len(chunks) <= 1

    # `--model` and `--max-retrieval` don't affect the config
    assert cargo_run(["config", "--get", "model"], stdout=True).strip() == "dummy"
    assert cargo_run(["config", "--get", "max_retrieval"], stdout=True).strip() == "5"

    # It's really tough to check whether the result is good or bad.
    # Let's just make sure that it doesn't panic, and dump the result to `result.json`.
    r = cargo_run(["retrieve-chunks", f"--model={test_model}", question, "--super-rerank"], stdout=True)
    send_message(f"--- {question} ---\n\n{r}")
