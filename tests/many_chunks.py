from random import randint, seed as rand_seed, shuffle
from subprocess import TimeoutExpired
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, rand_word, write_string

def many_chunks():
    rand_seed(0)
    goto_root()
    mk_and_cd_tmp_dir()
    txt_files = []
    tfidf_map = []

    for i in range(2000):
        file_name = f"{i:04}.txt"
        txt_files.append(file_name)

        if randint(0, 3) == 0:
            words = [rand_word() for _ in range(randint(500, 1000))]

        else:
            words = [rand_word() for _ in range(randint(5, 10))]

        write_string(file_name, " ".join(words))
        tfidf_map.append((words[0], file_name))

    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["config", "--set", "sleep_after_llm_call", "10"])
    cargo_run(["config", "--set", "chunk_size", "4000"])
    cargo_run(["add", *txt_files])
    cargo_run(["check"])
    break2 = False

    while True:
        # there are 2 cases to cover:
        # 1. implicit --auto-recover invoked by `rag build`
        # 2. explicit `rag check --auto-recover`
        cargo_run(["check", "--auto-recover"])

        for _ in range(4):
            try:
                cargo_run(["build"], timeout=5.0)

            except TimeoutExpired:
                pass

            else:
                break2 = True
                break

        if break2:
            break

    cargo_run(["remove", "0010.txt"])
    cargo_run(["check"])
    cargo_run(["add", "0010.txt"])
    cargo_run(["check"])
    cargo_run(["build"])
    cargo_run(["check"])

    # it takes too long to run tfidf thousands of times
    # TODO: run tfidf thousands times when K-V DB is implemented
    shuffle(tfidf_map)
    tfidf_map = tfidf_map[:20]

    for word, file_name in tfidf_map:
        assert file_name in cargo_run(["tfidf", word], stdout=True)
