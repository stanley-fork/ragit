import os
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, write_string

def recover():
    goto_root()
    mk_and_cd_tmp_dir()

    # step 0: init successfully
    cargo_run(["init"])
    cargo_run(["check"])
    cargo_run(["config", "--set", "model", "dummy"])

    # step 1: recover from a broken config file
    os.chdir(".ragit/configs")
    os.remove("query.json")
    os.chdir("../..")
    assert cargo_run(["check"], check=False) != 0
    cargo_run(["check", "--recover"])

    # `build.json` must be kept intact
    assert "dummy" in cargo_run(["config", "--get", "model"], stdout=True)

    # step 2: recover from a broken tfidf file
    write_string("test1.txt", "Hello, World!")
    write_string("test2.txt", "Good Bye, World!")
    cargo_run(["add", "test1.txt", "test2.txt"])
    cargo_run(["build"])
    cargo_run(["check"])

    for i in range(2):
        os.chdir(".ragit/chunks")
        assert len(dirs := [file for file in os.listdir()]) in [1, 2]
        os.chdir(dirs[0])
        assert len(tfidf_files := [file for file in os.listdir() if file.endswith(".tfidf")]) in [1, 2]

        # step 2.1: corrupt a tfidf file
        if i == 0:
            write_string(tfidf_files[0], "corrupted")

        # step 2.2: remove a tfidf file
        else:
            os.remove(tfidf_files[0])

        os.chdir("../../..")
        assert cargo_run(["check"], check=False) != 0
        cargo_run(["check", "--recover"])
        cargo_run(["build"])
        cargo_run(["check"])
        assert "test1.txt" in cargo_run(["tfidf", "hello"], stdout=True)
        assert "test2.txt" not in cargo_run(["tfidf", "hello"], stdout=True)
        assert "test1.txt" not in cargo_run(["tfidf", "good bye"], stdout=True)
        assert "test2.txt" in cargo_run(["tfidf", "good bye"], stdout=True)
