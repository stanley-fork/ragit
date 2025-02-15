from images import sample_markdown
import json
import os
import shutil
from utils import (
    cargo_run,
    count_chunks,
    count_files,
    count_images,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

def archive():
    goto_root()
    mk_and_cd_tmp_dir()

    # knowledge-base 1: a few files with a few images
    os.mkdir("base1")
    os.chdir("base1")
    shutil.copyfile("../../tests/images/empty.png", "sample2.png")
    shutil.copyfile("../../tests/images/empty.jpg", "sample5.jpg")
    shutil.copyfile("../../tests/images/empty.webp", "sample6.webp")
    write_string("sample1.md", sample_markdown)
    write_string("sample2.md", "hi! my name is baehyunsol")

    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["add", "sample1.md", "sample2.md"])
    cargo_run(["build"])
    archive_worker()

# call this function at a root dir of a knowledge base
# it'll move the cwd to `..` and return
def archive_worker():
    old_info = {
        "chunks": count_chunks(),
        "files": count_files(),
        "images": count_images(),
    }
    old_meta = json.loads(cargo_run(["meta", "--get-all", "--json"], stdout=True))
    cargo_run(["archive-create", "--output=../single.rag-archive"])
    cargo_run(["archive-create", "--size-limit=1048576", "--output=../splitted.rag-archive"])
    cargo_run(["archive-create", "--size-limit=1", "--output=../small-size.rag-archive"])

    # TODO: archive with more jobs
    # TODO: 1) modify a prompt, 2) archive with prompt, 3) restore the prompt, 4) check if the modified prompt is archived
    # TODO: archive configs

    os.chdir("..")
    cargo_run(["archive-extract", "--output=single-archive", "single.rag-archive"])
    splitted_archives = [a for a in os.listdir() if a.startswith("splitted.rag-archive")]
    small_archives = [a for a in os.listdir() if a.startswith("small-size.rag-archive")]
    cargo_run(["archive-extract", "--output=splitted-archive", *splitted_archives])
    cargo_run(["archive-extract", "--output=small-archive", *small_archives])

    extracted_archives = [
        "single-archive",
        "splitted-archive",
        "small-archive",
    ]

    for archive in extracted_archives:
        os.chdir(archive)
        cargo_run(["check"])
        new_info = {
            "chunks": count_chunks(),
            "files": count_files(),
            "images": count_images(),
        }
        new_meta = json.loads(cargo_run(["meta", "--get-all", "--json"]))

        if old_info != new_info:
            raise ValueError(f"old_info: {old_info}, new_info: {new_info}")

        if old_meta != new_meta:
            raise ValueError(f"old_meta: {old_meta}, new_meta: {new_meta}")

        os.chdir("..")
        shutil.rmtree(archive)
