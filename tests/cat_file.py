import json
from random import random
import shutil
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    rand_word,
    write_string,
)

def cat_file():
    goto_root()
    mk_and_cd_tmp_dir()
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["config", "--set", "chunk_size", "500"])
    cargo_run(["config", "--set", "slide_len", "150"])

    # step 1: See if `cat-file` dumps the exact content of files.
    #         We have to be careful: some file readers modify the
    #         content in order to give more context to LLMs.
    #         We have to choose file readers that do not modify
    #         files.
    files = {}

    for i in range(10):
        # for now, ragit chooses file reader based on file extensions
        for ext in ["md", "txt"]:
            file_name = f"{i}.{ext}"
            file_content = "".join([rand_word() + (" " if random() < 0.8 else "\n") for _ in range(i * 20)])
            write_string(file_name, file_content)
            files[file_name] = file_content
            cargo_run(["add", file_name])

    cargo_run(["build"])

    for file_name, file_content in files.items():
        content = cargo_run(["cat-file", file_name], stdout=True).strip()
        content_json = json.loads(cargo_run(["cat-file", "--json", file_name], stdout=True)).strip()
        assert content == file_content.strip()
        assert content == content_json

    # step 2: See if `cat-file` dumps the raw bytes of an image.
    shutil.copyfile("../tests/images/empty.png", "./empty.png")
    write_string("sample.md", "this is an image: ![sample](empty.png)")
    cargo_run(["add", "sample.md"])
    cargo_run(["build"])
    image_uid = cargo_run(["ls-images", "--uid-only"], stdout=True).strip()

    with open("empty.png", "rb") as f:
        assert f.read() == cargo_run(["cat-file", image_uid], stdout=True, raw_output=True)

    # step 3: See if `cat-file` on a text file with images dumps the uid of the image, not the raw bytes.
    assert image_uid in cargo_run(["cat-file", "sample.md"], stdout=True)
