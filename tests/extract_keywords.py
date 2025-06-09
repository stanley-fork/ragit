import json
import os
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir

def extract_keywords(test_model: str):
    goto_root()
    mk_and_cd_tmp_dir()
    cargo_run(["clone", "https://ragit.baehyunsol.com/sample/ragit"])
    os.chdir("ragit")
    cargo_run(["config", "--set", "model", test_model])

    # It's not mathmatically correct, but is practically correct.
    result = cargo_run(["extract-keywords", "How does ragit store chunks?"], stdout=True)
    assert "ragit" in result and "chunk" in result

    result = cargo_run(["extract-keywords", "--full-schema", "How does ragit store chunks?"], stdout=True)
    assert "ragit" in result and "chunk" in result

    result = json.loads(cargo_run(["extract-keywords", "--json", "How does ragit store chunks?"], stdout=True))
    assert any(["ragit" in x for x in result]) and any(["chunk" in x for x in result])

    result = json.loads(cargo_run(["extract-keywords", "--full-schema", "--json", "How does ragit store chunks?"], stdout=True))
    assert isinstance(result, dict)
    assert "keywords" in result and "extra" in result
    assert any(["ragit" in x for x in result["keywords"]]) and any(["chunk" in x for x in result["keywords"]])
