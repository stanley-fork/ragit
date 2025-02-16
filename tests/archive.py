from images import sample_markdown
import json
import os
from random import randint, seed as rand_seed
import shutil
from utils import (
    cargo_run,
    count_chunks,
    count_files,
    count_images,
    goto_root,
    mk_and_cd_tmp_dir,
    rand_word,
    read_string,
    write_string,
)

def archive():
    rand_seed(0)
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
    cargo_run(["meta", "--set", "test", "test"])
    archive_worker()

    # knowledge-base 2: a lot of files with long texts
    os.mkdir("base2")
    os.chdir("base2")
    files = []

    for i in range(300):
        write_string(f"{i:03}.txt", " ".join([rand_word() for _ in range(randint(300, 600))]))
        files.append(f"{i:03}.txt")

    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["add", *files])
    cargo_run(["build"])
    cargo_run(["meta", "--set", "test", "test2"])
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
    old_chunk_size = eval(cargo_run(["config", "--get", "chunk_size"], stdout=True))
    new_chunk_size = old_chunk_size + 1
    cargo_run(["config", "--set", "chunk_size", str(new_chunk_size)])
    old_prompt = read_string(".ragit/prompts/raw.pdl")
    new_prompt = "this is the new prompt"
    write_string(".ragit/prompts/raw.pdl", new_prompt)

    cargo_run(["archive-create", "--output=../single.rag-archive", "--no-prompts", "--no-configs"])
    cargo_run(["archive-create", "--size-limit=1048576", "--output=../splitted.rag-archive", "--no-prompts", "--no-configs"])
    cargo_run(["archive-create", "--size-limit=1", "--output=../small-size.rag-archive", "--no-prompts", "--no-configs"])
    cargo_run(["archive-create", "--output=../configs.rag-archive", "--no-prompts", "--configs"])
    cargo_run(["archive-create", "--output=../prompts.rag-archive", "--prompts", "--no-configs"])

    # cannot overwrite
    assert cargo_run(["archive-create", "--output=../single.rag-archive", "--no-prompts", "--no-configs"], check=False) != 0

    # forcefully overwrite
    cargo_run(["archive-create", "--output=../single.rag-archive", "--no-prompts", "--no-configs", "--force"])

    os.chdir("..")
    archives = {
        "single-archive": ["single.rag-archive"],
        "configs-archive": ["configs.rag-archive"],
        "prompts-archive": ["prompts.rag-archive"],
        "splitted-archive": [a for a in os.listdir() if a.startswith("splitted.rag-archive")],
        "small-archive": [a for a in os.listdir() if a.startswith("small-size.rag-archive")],
    }

    for dir, archive_files in archives.items():
        cargo_run(["archive-extract", "--output", dir, *archive_files])

        # cannot overwrite
        assert cargo_run(["archive-extract", "--output", dir, *archive_files], check=False) != 0

        # forcefully overwrite
        cargo_run(["archive-extract", "--force", "--output", dir, *archive_files])

        for archive_file in archive_files:
            os.remove(archive_file)

    extracted_archives = [
        ("single-archive", old_chunk_size, old_prompt),
        ("configs-archive", new_chunk_size, old_prompt),
        ("prompts-archive", old_chunk_size, new_prompt),
        ("splitted-archive", old_chunk_size, old_prompt),
        ("small-archive", old_chunk_size, old_prompt),
    ]

    for (archive, chunk_size, prompt) in extracted_archives:
        os.chdir(archive)
        cargo_run(["check"])
        new_info = {
            "chunks": count_chunks(),
            "files": count_files(),
            "images": count_images(),
        }
        new_meta = json.loads(cargo_run(["meta", "--get-all", "--json"], stdout=True))
        chunk_size_ = eval(cargo_run(["config", "--get", "chunk_size"], stdout=True))
        prompt_ = read_string(".ragit/prompts/raw.pdl")

        if old_info != new_info:
            raise ValueError(f"old_info: {old_info}, new_info: {new_info}")

        if old_meta != new_meta:
            raise ValueError(f"old_meta: {old_meta}, new_meta: {new_meta}")

        if chunk_size != chunk_size_:
            raise ValueError(f"expected: {chunk_size}, got: {chunk_size_}")

        if prompt != prompt_:
            raise ValueError(f"expected: {prompt}, got: {prompt_}")

        os.chdir("..")
        shutil.rmtree(archive)
