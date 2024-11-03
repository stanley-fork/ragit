import os
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, write_string

def auto_recover():
    goto_root()
    mk_and_cd_tmp_dir()

    # step 0: init successfully
    cargo_run(["init"])
    cargo_run(["check"])
    cargo_run(["config", "--set", "model", "dummy"])

    # step 1: recover from broken config files
    os.chdir(".rag_index/configs")
    os.remove("query.json")
    os.chdir("../..")
    assert cargo_run(["check"], check=False) != 0
    cargo_run(["check", "--auto-recover"])

    # `build.json` must be kept intact
    assert "dummy" in cargo_run(["config", "--get", "model"], stdout=True)

    # step 2: recover from broken chunk files
    write_string("test.txt", "Hello, World!")
    cargo_run(["add", "test.txt"])
    cargo_run(["build"])
    cargo_run(["check"])

    # step 2.1: remove `.chunks` file and recover
    os.chdir(".rag_index/chunks")
    assert len((chunk_files := [file for file in os.listdir() if file.endswith("chunks")])) == 1
    os.remove(chunk_files[0])
    os.chdir("../..")
    assert cargo_run(["check"], check=False) != 0
    cargo_run(["check", "--auto-recover"])
    cargo_run(["add", "test.txt"])
    cargo_run(["build"])
    cargo_run(["check"])

    # step 2.2: remove `.tfidf` file and recover
    os.chdir(".rag_index/chunks")
    assert len((tfidf_files := [file for file in os.listdir() if file.endswith("tfidf")])) == 1
    os.remove(tfidf_files[0])
    os.chdir("../..")
    assert cargo_run(["check"], check=False) != 0
    cargo_run(["check", "--auto-recover"])
    cargo_run(["add", "test.txt"])
    cargo_run(["build"])
    cargo_run(["check"])

    # step 2.3: remove chunk_index file and recover
    os.chdir(".rag_index/chunks_index")
    assert len((chunk_index_files := [file for file in os.listdir() if file.endswith("json")])) > 0

    for chunk_index_file in chunk_index_files:
        os.remove(chunk_index_file)

    os.chdir("../..")
    assert cargo_run(["check"], check=False) != 0
    cargo_run(["check", "--auto-recover"])
    cargo_run(["add", "test.txt"])
    cargo_run(["build"])
    cargo_run(["check"])
