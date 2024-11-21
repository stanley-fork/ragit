import os
import shutil
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, write_string

def recover():
    goto_root()
    mk_and_cd_tmp_dir()

    # step 0: init successfully
    cargo_run(["init"])
    cargo_run(["check"])
    cargo_run(["config", "--set", "model", "dummy"])

    # step 1: recover from broken config files
    os.chdir(".ragit/configs")
    os.remove("query.json")
    os.chdir("../..")
    assert cargo_run(["check"], check=False) != 0
    cargo_run(["check", "--recover"])

    # `build.json` must be kept intact
    assert "dummy" in cargo_run(["config", "--get", "model"], stdout=True)

    # step 2: recover after breaking a tfidf file
    write_string("test.txt", "Hello, World!")
    cargo_run(["add", "test.txt"])
    cargo_run(["build"])
    cargo_run(["check"])
    cargo_run(["tfidf", "123"])  # make sure that the tfidf file is created
    os.chdir(".ragit/chunks")
    assert len(dirs := [file for file in os.listdir()]) == 1
    os.chdir(dirs[0])
    assert len(tfidf_files := [file for file in os.listdir() if file.endswith(".tfidf")]) == 1
    write_string(tfidf_files[0], "corrupted")
    os.chdir("../../..")
    assert cargo_run(["check"], check=False) != 0
    cargo_run(["check", "--recover"])
    cargo_run(["build"])
    cargo_run(["check"])
