import json
import os
import shutil
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir

def images3(test_model: str):
    goto_root()
    mk_and_cd_tmp_dir()

    shutil.copyfile("../tests/images/empty.png", "empty.png")
    shutil.copyfile("../tests/images/empty.jpg", "empty.jpg")
    shutil.copyfile("../tests/images/empty.webp", "empty.webp")
    shutil.copyfile("../tests/images/hello_world.webp", "hello_world.webp")

    cargo_run(["init"])
    cargo_run(["config", "--set", "strict_file_reader", "true"])
    cargo_run(["config", "--set", "summary_after_build", "false"])
    cargo_run(["config", "--set", "model", test_model])

    for image in os.listdir():
        cargo_run(["add", image])

    cargo_run(["check"])
    cargo_run(["build"])
    cargo_run(["check"])

    image = json.loads(cargo_run(["ls-images", "hello_world.webp", "--json"], stdout=True))
    extracted_text = image[0]["extracted_text"].lower()
    assert "hello" in extracted_text
    assert "world" in extracted_text
