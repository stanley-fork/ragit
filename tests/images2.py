import json
import os
import re
import shutil
from utils import (
    cargo_run,
    count_images,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

def images2(test_model: str):
    goto_root()
    mk_and_cd_tmp_dir()

    # step 0: initialize knowledge-base
    write_string("sample.md", "This is a text on an wooden plank: ![](sample.webp)")
    write_string("sample2.md", "You'll not get this from tfidf search.")

    # make sure that the name of the copied file does NOT contain "hello" or "world"
    shutil.copyfile("../tests/images/hello_world.webp", "sample.webp")

    cargo_run(["init"])
    cargo_run(["config", "--set", "model", test_model])
    cargo_run(["config", "--set", "strict_file_reader", "true"])
    cargo_run(["config", "--set", "dump_log", "true"])
    cargo_run(["add", "sample.md", "sample2.md"])
    cargo_run(["check"])
    cargo_run(["build"])
    os.chdir(".ragit/images")
    assert len(inner_dir := os.listdir()) == 1
    os.chdir(inner_dir[0])
    json_files = [f for f in os.listdir() if f.endswith(".json")]
    assert len(json_files) == 1

    with open(json_files[0], "r") as f:
        j = json.load(f)
        extracted_text = j['extracted_text'].lower()

    assert "hello" in extracted_text
    assert "world" in extracted_text

    os.chdir("../../..")

    # step 1: tfidf
    search_result = cargo_run(["tfidf", "hello world"], stdout=True)
    assert "sample.md" in search_result
    assert "sample2.md" not in search_result

    # step 2: ls-images
    assert count_images() == 1
    ls_file_result = cargo_run(["ls-files", "sample.md"], stdout=True)
    assert "1 total files" in ls_file_result
    file_uid = re.search(r"uid\:\s([0-9a-f]{64})", ls_file_result).group(1)

    ls_image_result = cargo_run(["ls-images", file_uid], stdout=True)
    assert "1 images" in ls_image_result
    assert "hello" in ls_image_result.lower()
    assert "world" in ls_image_result.lower()
