import json
import os
import shutil
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, write_string

def images2(test_model: str):
    goto_root()
    mk_and_cd_tmp_dir()
    write_string("sample.md", "This is a text on an wooden plank: ![](sample.webp)")
    write_string("sample2.md", "You'll not get this from tfidf search.")
    shutil.copyfile("../tests/images/hello_world.webp", "sample.webp")

    cargo_run(["init"])
    cargo_run(["config", "--set", "model", test_model])
    cargo_run(["config", "--set", "strict_file_reader", "true"])
    cargo_run(["config", "--set", "dump_log", "true"])
    cargo_run(["add", "sample.md"])
    cargo_run(["check"])
    cargo_run(["build"])
    os.chdir(".rag_index/images")
    json_files = [f for f in os.listdir() if f.endswith(".json")]
    assert len(json_files) == 1

    with open(json_files[0], "r") as f:
        j = json.load(f)
        extracted_text = j['extracted_text'].lower()

    assert "hello" in extracted_text
    assert "world" in extracted_text

    os.chdir("../..")
    search_result = cargo_run(["tfidf", "hello world"], stdout=True)
    assert "sample.md" in search_result
    assert "sample2.md" not in search_result
