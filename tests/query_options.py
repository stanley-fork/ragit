from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

def query_options(test_model: str):
    goto_root()
    mk_and_cd_tmp_dir()
    cargo_run(["init"])
    write_string("story.txt", "There lived a man named Baehyunsol. He lived in ragit city.")
    cargo_run(["config", "--set", "summary_after_build", "false"])
    cargo_run(["config", "--set", "model", test_model])
    cargo_run(["add", "story.txt"])
    cargo_run(["build"])

    assert "ragit" in cargo_run(["query", "In which city did Baehyunsol live?"], stdout=True).lower()
    assert "ragit" not in cargo_run(["query", "--disable-rag", "In which city did Baehyunsol live?"], stdout=True).lower()
    assert "ragit" not in cargo_run(["query", "--max-summaries=0", "In which city did Baehyunsol live?"], stdout=True).lower()
    assert "ragit" not in cargo_run(["query", "--max-retrieval=0", "In which city did Baehyunsol live?"], stdout=True).lower()

    # Let's make sure that the options are temporary and does not affect the config.
    assert "ragit" in cargo_run(["query", "In which city did Baehyunsol live?"], stdout=True).lower()

    assert cargo_run(["query", "--model=error", "It should fail."], check=False) != 0
    assert "dummy" in cargo_run(["query", "--model=dummy", "Hello, World!"], stdout=True)
