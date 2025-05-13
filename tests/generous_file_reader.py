import json
import random
import shutil
from utils import (
    cargo_run,
    count_files,
    goto_root,
    mk_and_cd_tmp_dir,
    rand_word,
    write_string,
)

def generous_file_reader():
    goto_root()
    mk_and_cd_tmp_dir()

    write_string("wrong-extension.png", "This is not a png!")
    shutil.copyfile("../tests/images/empty.webp", "wrong-extension.svg")
    shutil.copyfile("../tests/images/empty.jpg", "wrong-extension.txt")

    write_string("invalid-image-url.md", "Do you see this? ![](https://invalid/url.png) I guess not...")
    write_string("wrong-extension-1.md", "This seems like a png, but is a text: ![](wrong-extension.png)")
    write_string("wrong-extension-2.md", "This seems like a png, but is a text: ![](wrong-extension.svg)")
    invalid_files = [
        "wrong-extension.png",
        "wrong-extension.svg",
        "wrong-extension.txt",
        "wrong-extension-1.md",
        "wrong-extension-2.md",
        "invalid-image-url.md",
    ]
    valid_files = []

    for i in range(200):
        write_string(f"{i}.txt", " ".join([rand_word() for _ in range(20)]))
        valid_files.append(f"{i}.txt")

    files = invalid_files + valid_files
    random.shuffle(files)  # for more real-word-like case

    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["config", "--set", "strict_file_reader", "true"])
    cargo_run(["add", *files])
    cargo_run(["build"])
    cargo_run(["check"])
    staged_files = json.loads(cargo_run(["ls-files", "--staged", "--json", "--name-only"], stdout=True))
    processed_files = json.loads(cargo_run(["ls-files", "--processed", "--json", "--name-only"], stdout=True))
    assert set(invalid_files) == set(staged_files)
    assert set(valid_files) == set(processed_files)

    cargo_run(["config", "--set", "strict_file_reader", "false"])
    cargo_run(["rm", "--all"])
    cargo_run(["add", *files])
    cargo_run(["build"])
    cargo_run(["check"])
    assert count_files() == (len(files), 0, len(files))  # (total, staged, processed)
    assert "invalid/url.png" in cargo_run(["cat-file", "invalid-image-url.md"], stdout=True)
    assert "wrong-extension.png" in cargo_run(["cat-file", "wrong-extension-1.md"], stdout=True)
    assert "wrong-extension.svg" in cargo_run(["cat-file", "wrong-extension-2.md"], stdout=True)
