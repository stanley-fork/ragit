import os
import shutil
from random import randint, seed as rand_seed
import re
from subprocess import TimeoutExpired
from utils import cargo_run, count_chunks, count_files, goto_root

def end_to_end(test_model: str):
    rand_seed(0)
    goto_root()
    os.chdir("docs")
    md_files = []
    txt_files = []

    for file in os.listdir():
        if not file.endswith(".md"):
            continue

        md_files.append(file)

    for file in os.listdir("commands"):
        if not file.endswith(".txt"):
            continue

        txt_files.append(os.path.join("commands", file))

    if ".ragit" in os.listdir():
        cargo_run(["reset", "--hard"])

    assert len(md_files) > 2  # `rag build` has to take at least 5 seconds
    cargo_run(["init"])
    cargo_run(["check"])

    # step 1: set/get config
    assert cargo_run(["config", "--set", "model", "invalid-model-name"], check=False) != 0
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["check"])
    assert "dummy" in cargo_run(["config", "--get", "model"], stdout=True)
    cargo_run(["config", "--set", "model", test_model])
    assert test_model in cargo_run(["config", "--get", "model"], stdout=True)
    cargo_run(["config", "--set", "sleep_after_llm_call", "null"])
    cargo_run(["config", "--set", "sleep_after_llm_call", "2000"])
    assert cargo_run(["config", "--set", "sleep_after_llm_call", "this_is_not_a_number"], check=False) != 0
    assert "2000" in cargo_run(["config", "--get", "sleep_after_llm_call"], stdout=True)
    cargo_run(["config", "--set", "dump_log", "true"])
    cargo_run(["config", "--set", "strict_file_reader", "true"])
    cargo_run(["check"])

    # step 1.1: the commands shall run anywhere inside the repo
    os.mkdir("tmp")
    os.chdir("tmp")
    assert test_model in cargo_run(["config", "--get", "model"], stdout=True)
    cargo_run(["check"])
    os.chdir("..")
    shutil.rmtree("tmp")

    # step 1.2: `rag config --get-all` itself has an assert statement, which checks whether there's a key collision
    cargo_run(["config", "--get-all"])

    # step 2: add the files
    cargo_run(["add", *md_files])
    cargo_run(["add", *txt_files])
    cargo_run(["check"])
    file_count, _, _ = count_files()

    assert file_count == len(md_files) + len(txt_files)

    # step 2.1: rm all the files and add the files again
    cargo_run(["rm", *md_files])
    cargo_run(["rm", *txt_files])
    cargo_run(["check"])
    file_count, _, _ = count_files()

    assert file_count == 0

    cargo_run(["add", *md_files])
    cargo_run(["add", *txt_files])
    cargo_run(["check"])
    file_count, _, _ = count_files()

    assert file_count == len(md_files) + len(txt_files)

    # step 3: build: pause and resume
    try:
        cargo_run(["build"], timeout=8.0 + randint(0, 8))

    except TimeoutExpired:
        pass

    else:
        raise Exception("The build should have timed out")

    cargo_run(["check", "--recover"])
    cargo_run(["config", "--set", "sleep_after_llm_call", "null"])
    cargo_run(["build"])
    cargo_run(["check"])

    # running `rag build` after the knowledge-base built does nothing
    cargo_run(["build"])
    cargo_run(["check"])

    # step 4: ls-chunks
    chunks = cargo_run(["ls-chunks"], stdout=True)
    chunk_uids = []

    for line in chunks.split("\n"):
        if (r := re.match(r"^uid\:\s([0-9a-f]{32,})$", line)) is not None:
            chunk_uids.append(r.group(1))

    # step 5: check whether tfidf index has token "ragit"
    has_ragit_in_tfidf = False

    for chunk_uid in chunk_uids:
        tfidf_dump = cargo_run(["ls-terms", chunk_uid], stdout=True)
        has_ragit_in_tfidf = has_ragit_in_tfidf or "ragit" in tfidf_dump

    assert has_ragit_in_tfidf

    # step 6: ls commands
    file_count_prev, _, _ = count_files()
    chunk_count_prev = count_chunks()

    assert file_count_prev == len(md_files) + len(txt_files)

    # step 7: rm
    cargo_run(["rm", md_files[0]])
    cargo_run(["check"])
    file_count_next, _, _ = count_files()
    chunk_count_next = count_chunks()

    assert file_count_prev == file_count_next + 1
    assert chunk_count_prev > chunk_count_next

    # step 8: add again
    cargo_run(["add", md_files[0]])
    cargo_run(["check"])
    file_count, _, _ = count_files()
    chunk_count = count_chunks()

    assert file_count_prev == file_count
    assert chunk_count_prev > chunk_count  # `rag build` is not run yet

    cargo_run(["build"])
    cargo_run(["check"])
    chunk_count = count_chunks()

    assert chunk_count_prev == chunk_count

    # step 9: multiple `add` operations with different flags
    cargo_run(["add", "--ignore", md_files[0]])
    cargo_run(["check"])
    chunk_count_new = count_chunks()

    assert chunk_count == chunk_count_new

    cargo_run(["add", "--auto", md_files[0]])
    cargo_run(["check"])
    chunk_count_new = count_chunks()

    assert chunk_count == chunk_count_new

    cargo_run(["add", "--force", md_files[0]])
    cargo_run(["check"])
    chunk_count_new = count_chunks()

    assert chunk_count > chunk_count_new

    cargo_run(["build"])
    cargo_run(["check"])
    chunk_count_new = count_chunks()

    assert chunk_count == chunk_count_new

    # step 10: break the knowledge-base and run `--recover`
    os.chdir(".ragit/files")
    assert len(os.listdir()) > 0

    for file_index in os.listdir():
        shutil.rmtree(file_index)

    os.chdir("../..")
    assert cargo_run(["check"], check=False) != 0
    cargo_run(["check", "--recover"])
    cargo_run(["check"])

    # step 11: query
    cargo_run(["gc", "--logs"])
    cargo_run(["query", "What makes ragit special?"])
