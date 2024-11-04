import os
import shutil
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, write_string

def images2(test_model: str):
    goto_root()
    mk_and_cd_tmp_dir()
    write_string("sample.md", "This is a text editor: ![](sample.png)")
    shutil.copyfile("../tests/images/hello_world.png", "sample.png")

    cargo_run(["init"])
    cargo_run(["config", "--set", "model", test_model])
    cargo_run(["config", "--set", "strict_file_reader", "true"])
    cargo_run(["add", "sample.md"])
    cargo_run(["check"])
    cargo_run(["build"])
    os.chdir(".rag_index/images")
    json_files = [f for f in os.listdir() if f.endswith(".json")]
    assert len(json_files) == 1

    with open(json_files[0], "r") as f:
        extracted_text = f['extracted_text'].lower()

    assert "hello" in extracted_text
    assert "world" in extracted_text
