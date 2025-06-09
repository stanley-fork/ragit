import os
import re
import shutil
from utils import (
    cargo_run,
    goto_root,
    ls_recursive,
    mk_and_cd_tmp_dir,
    rand_word,
    write_string,
)

def ii():
    goto_root()
    mk_and_cd_tmp_dir()

    # step 1: create a local knowledge-base
    os.mkdir("base")
    os.chdir("base")
    shutil.copytree("../../src", "src")
    os.chdir("src")

    if ".ragit" in os.listdir():
        shutil.rmtree(".ragit")

    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["config", "--set", "chunk_size", "512"])
    cargo_run(["config", "--set", "slide_len", "128"])
    cargo_run(["config", "--set", "enable_ii", "false"])
    assert cargo_run(["ii-status"], stdout=True).strip() == "not initialized"
    cargo_run(["add", *(ls_recursive("rs") + ls_recursive("txt"))])

    cargo_run(["build"])
    assert cargo_run(["ii-status"], stdout=True).strip() == "not initialized"
    ii_worker()
    os.chdir("..")

    # step 2: clone remote knowledge-bases
    for (url, dir) in [
        ("https://ragit.baehyunsol.com/sample/git", "git"),
        ("https://ragit.baehyunsol.com/sample/ragit", "ragit"),
        ("https://ragit.baehyunsol.com/sample/rustc", "rustc"),
    ]:
        cargo_run(["clone", url, dir])
        os.chdir(dir)
        cargo_run(["config", "--set", "enable_ii", "false"])
        cargo_run(["config", "--set", "model", "dummy"])
        ii_worker()
        os.chdir("..")

def ii_worker():
    # Strategy:
    # 1. There are an arbitrary number of chunks that contain `terms`.
    #    But let's make sure that some terms appear very often and some
    #    terms appear very rarely, for better test coverage.
    # 2. It runs `tfidf <term>` for each `term` in `terms`.
    #    Each time, it records the rank of the chunks.
    # 3. After running `ii-build`, it runs `tfidf <term>` again.
    #    Again it records the rank of the chunks.
    # 4. Result from 2 is the answer and result from 3 is an 'approximation'.
    #    It's goal is to make sure that the approximation is close enough.
    # 5. Rules
    #    - If answer has >= 10 chunks and approximation has >= 10 chunks,
    #      - The top 3 of each set must be included in the top 10 of the other.
    #    - If answer has < 10 chunks and approximation has < 10 chunks,
    #      - The set of the chunks must be the same.
    #    - Otherwise,
    #      - Error
    terms = generate_terms()
    answers = {}
    approximations = {}

    for term in terms:
        uids = cargo_run(["tfidf", term, "--uid-only", "--limit=10"], stdout=True)
        uids = [uid for uid in uids.split("\n") if re.match(r"^[0-9a-z]{64}$", uid)]
        assert len(uids) <= 10
        answers[term] = uids

    cargo_run(["ii-build"])
    cargo_run(["check"])
    assert cargo_run(["ii-status"], stdout=True).strip() == "complete"
    cargo_run(["config", "--set", "enable_ii", "true"])

    for term in terms:
        uids = cargo_run(["tfidf", term, "--uid-only", "--limit=10"], stdout=True)
        uids = [uid for uid in uids.split("\n") if re.match(r"^[0-9a-z]{64}$", uid)]
        assert len(uids) <= 10
        approximations[term] = uids

    for term in terms:
        answer = answers[term]
        approximation = approximations[term]

        try:
            if len(answer) == 10:
                if len(approximation) == 10:
                    for i in range(3):
                        if answer[i] not in approximation:
                            raise AssertionError(f"answer[{i}] not in approximation")

                        if approximation[i] not in answer:
                            raise AssertionError(f"approximation[{i}] not in answer")

                else:
                    raise AssertionError(f"len(answer) == 10, len(approximation) == {len(approximation)}")

            else:
                if len(approximation) == 10:
                    raise AssertionError(f"len(answer) == {len(answer)}, len(approximation) == 10")

                elif set(answer) != set(approximation):
                    raise AssertionError(f"set(answer) != set(approximation)")

        except AssertionError as e:
            raise AssertionError(f"tfidf result on term '{term}' is not close enough. error: `{e}`, answer: {answer}, approximation: {approximation}")

    # incremental update of ii
    write_string("self-introduction.txt", "Hi, my name is baehyunsol.")
    cargo_run(["add", "self-introduction.txt"])
    assert cargo_run(["ii-status"], stdout=True).strip() == "complete"
    cargo_run(["build"])
    cargo_run(["check"])
    assert cargo_run(["ii-status"], stdout=True).strip() == "complete"
    assert "self-introduction.txt" in cargo_run(["tfidf", "Hi, my name is baehyunsol."], stdout=True)

def generate_terms():
    dictionary = []
    words = set()

    for line in cargo_run(["ls-terms"], stdout=True).split("\n"):
        if (r := re.match(r"^\s*\"([0-9a-zA-Z가-힣]+)\"\:\s*(\d+)$", line)) is not None:
            dictionary.append((r.group(1), int(r.group(2))))
            words.add(r.group(1))

    dictionary.sort(key=lambda x: x[0])  # For deterministic result
    dictionary.sort(key=lambda x: x[1], reverse=True)
    dictionary = [term for term, _ in dictionary]

    if len(dictionary) < 500:
        raise Exception("The dataset is not big enough. Please make sure that there are more than 500 terms.")

    very_frequent_terms = dictionary[:5]
    frequent_terms = dictionary[len(dictionary) // 20:len(dictionary) // 20 + 5]
    less_frequent_terms = dictionary[len(dictionary) // 2:len(dictionary) // 2 + 5]
    rare_terms = dictionary[-5:]
    never_terms = []

    while len(never_terms) < 5:
        term = rand_word()

        if term not in words:
            never_terms.append(term)

    return [
        *frequent_terms,
        *less_frequent_terms,
        *rare_terms,
        *never_terms,
        " ".join(frequent_terms),
        " ".join(less_frequent_terms),
        " ".join(rare_terms),
        *[" ".join([very_frequent_terms[i], less_frequent_terms[i], rare_terms[i]]) for i in range(5)],
        *[" ".join([very_frequent_terms[i], less_frequent_terms[i]]) for i in range(5)],
    ]
