import os
from random import randint, seed as rand_seed
from subprocess import TimeoutExpired
from utils import (
    cargo_run,
    clean,
    count_files,
    goto_root,
    mk_and_cd_tmp_dir,
    rand_word,
    write_string,
)

def external_bases():
    rand_seed(0)
    goto_root()
    mk_and_cd_tmp_dir()
    os.mkdir("root")
    os.chdir("root")
    cargo_run(["init"])
    prefixes = {}
    base_count = randint(3, 8)

    for i in range(base_count):
        dir_name = f"base_{i}"
        os.mkdir(dir_name)
        os.chdir(dir_name)
        cargo_run(["init"])
        cargo_run(["check"])
        cargo_run(["config", "--set", "model", "dummy"])
        cargo_run(["config", "--set", "sleep_after_llm_call", "200"])
        cargo_run(["config", "--set", "chunk_size", "8000"])
        cargo_run(["config", "--set", "strict_file_reader", "true"])
        file_count = randint(3, 8)

        for j in range(file_count):
            file_name = f"base_{i}_doc_{j}.txt"
            long_doc = " ".join([rand_word() for _ in range(randint(2000, 8000))])
            prefix = long_doc[:16]  # let's assume it's unique
            prefixes[prefix] = file_name
            write_string(file_name, long_doc)

            cargo_run(["add", "--auto", file_name])
            cargo_run(["check"])

        try:
            cargo_run(["build"], timeout=1.0)

        except TimeoutExpired:
            pass

        else:
            raise Exception("The build should have timed out")

        cargo_run(["check", "--auto-recover"])
        cargo_run(["config", "--set", "sleep_after_llm_call", "0"])
        cargo_run(["check"])
        cargo_run(["build"])
        cargo_run(["check"])
        _, _, processed_files = count_files()
        assert processed_files == file_count

        os.chdir("..")
        cargo_run(["ext", dir_name])
        cargo_run(["check", "--recursive"])

    for prefix, file in prefixes.items():
        tfidf_result = cargo_run(["tfidf", prefix], stdout=True)
        assert file in tfidf_result

    os.chdir("../..")
    clean()
