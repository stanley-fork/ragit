import os
from random import seed as rand_seed
import shutil
from utils import (
    cargo_run,
    count_chunks,
    count_files,
    goto_root,
    mk_and_cd_tmp_dir,
    rand_word,
    write_string,
)

def merge():
    rand_seed(0)
    goto_root()
    mk_and_cd_tmp_dir()
    docs = [" ".join([rand_word() for _ in range(1000)]) for _ in range(7)]
    docs[-1] += "\n\n![image](empty.png)"
    terms_map = {doc.split(" ")[0]: f"doc_{i}.md" for i, doc in enumerate(docs)}

    # base1: a base with 7 documents
    os.mkdir("base1")
    os.chdir("base1")
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["config", "--set", "strict_file_reader", "true"])
    shutil.copy2("../../tests/images/empty.png", "empty.png")

    for i, doc in enumerate(docs):
        write_string(f"doc_{i}.md", doc)
        cargo_run(["add", f"doc_{i}.md"])

    cargo_run(["build"])
    cargo_run(["check"])
    chunk_count = count_chunks()
    os.chdir("..")

    # sub-base1: a base with first 3 documents
    os.mkdir("sub-base1")
    os.chdir("sub-base1")
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])

    for i, doc in enumerate(docs[:3]):
        write_string(f"doc_{i}.md", doc)
        cargo_run(["add", f"doc_{i}.md"])

    cargo_run(["build"])
    cargo_run(["check"])
    os.chdir("..")

    # sub-base2: a base with last 4 documents
    os.mkdir("sub-base2")
    os.chdir("sub-base2")
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    shutil.copy2("../../tests/images/empty.png", "empty.png")

    for i, doc in enumerate(docs[3:]):
        write_string(f"doc_{i + 3}.md", doc)
        cargo_run(["add", f"doc_{i + 3}.md"])

    cargo_run(["build"])
    cargo_run(["check"])
    os.chdir("..")

    # base2: merge of sub-base1 and sub-base2, without prefix
    os.mkdir("base2")
    os.chdir("base2")
    cargo_run(["init"])
    cargo_run(["merge", "../sub-base1"])
    cargo_run(["merge", "../sub-base2"])
    cargo_run(["check"])

    # some checks
    assert count_files() == (7, 0, 7)
    assert count_chunks() == chunk_count

    for i in range(7):
        # some docs include images, which cannot be handled by `cat-file`. so we compare only the first 1000 characters
        assert cargo_run(["cat-file", f"doc_{i}.md"], stdout=True).strip()[:1000] == docs[i][:1000]

    for _ in range(2):
        for term, doc in terms_map.items():
            tfidf_result = cargo_run(["tfidf", term], stdout=True)
            assert doc in tfidf_result

            for another_doc in terms_map.values():
                if another_doc == doc:
                    continue

                assert another_doc not in tfidf_result

        cargo_run(["ii-build"])
        cargo_run(["check"])

    os.chdir("..")

    # base3: merge of sub-base1 and sub-base2, with different prefixes
    os.mkdir("base3")
    os.chdir("base3")
    cargo_run(["init"])
    cargo_run(["merge", "../sub-base1", "--prefix", "sub1"])
    cargo_run(["merge", "../sub-base2", "--prefix=sub2"])
    cargo_run(["check"])

    # some checks
    assert count_files() == (7, 0, 7)
    assert count_chunks() == chunk_count

    for i in range(7):
        # some docs include images, which cannot be handled by `cat-file`. so we compare only the first 1000 characters
        assert cargo_run(["cat-file", f"sub{min(i // 3 + 1, 2)}/doc_{i}.md"], stdout=True).strip()[:1000] == docs[i][:1000]

    for _ in range(2):
        for term, doc in terms_map.items():
            tfidf_result = cargo_run(["tfidf", term], stdout=True)
            assert doc in tfidf_result

            for another_doc in terms_map.values():
                if another_doc == doc:
                    continue

                assert another_doc not in tfidf_result

        cargo_run(["ii-build"])
        cargo_run(["check"])

    # merging the same base with different prefixes should be fine
    cargo_run(["merge", "../sub-base1", "--prefix", "sub3"])
    cargo_run(["check"])
    assert count_files() == (10, 0, 10)

    # merging the same base with the same prefix should fail
    assert cargo_run(["merge", "../sub-base1", "--prefix", "sub1", "--reject"], check=False) != 0
    cargo_run(["check"])

    # a failed merge should not affect the base
    assert count_files() == (7, 0, 7)

    # with `--force`, the merge should be successful
    cargo_run(["merge", "../sub-base1", "--prefix", "sub1", "--force"])
    cargo_run(["check"])

    # a failed merge should not affect the base
    assert count_files() == (7, 0, 7)
