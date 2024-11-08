from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, rand_word, write_string
from random import randint, seed as rand_seed, shuffle

def many_chunks():
    rand_seed(0)
    goto_root()
    mk_and_cd_tmp_dir()
    txt_files = []
    tfidf_map = []

    for i in range(randint(1000, 2000)):
        file_name = f"{i:04}.txt"
        txt_files.append(file_name)
        words = [rand_word() for _ in range(randint(3, 8))]
        write_string(file_name, " ".join(words))
        tfidf_map.append((words[0], file_name))

    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["add", *txt_files])
    cargo_run(["check"])
    cargo_run(["build"])
    cargo_run(["check"])

    cargo_run(["remove", "0010.txt"])
    cargo_run(["check"])
    cargo_run(["add", "0010.txt"])
    cargo_run(["check"])
    cargo_run(["build"])
    cargo_run(["check"])

    # it takes too long to run tfidf thousands of times
    shuffle(tfidf_map)
    tfidf_map = tfidf_map[:20]

    for word, file_name in tfidf_map:
        assert file_name in cargo_run(["tfidf", word], stdout=True)
