import os
import re
from utils import cargo_run, goto_root

# In order to test invert indexes, we need a large enough dataset,
# which is not randomly generated, and easy to fetch. For now, it's
# using `docs/*.md` files, but need a bigger dataset.
def ii():
    goto_root()
    os.chdir("docs")

    if ".ragit" in os.listdir():
        cargo_run(["reset", "--hard"])

    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["config", "--set", "chunk_size", "512"])
    cargo_run(["config", "--set", "slide_len", "128"])
    cargo_run(["config", "--set", "enable_ii", "false"])

    for file in os.listdir():
        if file.endswith(".md") and "prompt" not in file:
            cargo_run(["add", file])

    cargo_run(["build"])

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
    #    - Answer has >= 10 chunks, Approximation has >= 10 chunks.
    #      - The top 3 of each other must be included in the top 10 of the other.
    #    - Answer has < 10 chunks, Approximation has < 10 chunks.
    #      - The set of chunks must be the same.
    #    - Otherwise,
    #      - Error
    terms = [
        "",
        "pdf", "media", "ragit", "core", "prompt",
        "pdf, media",
        "ragit, core",
        "ragit, rag",
        "prompt, engineer, auto",
        "verylongstring", "설마 이런 단어는 안 나오겠지?", "🦫",
    ]
    answers = {}
    approximations = {}

    for term in terms:
        uids = cargo_run(["tfidf", term, "--uid-only"], stdout=True)
        uids = [uid for uid in uids.split("\n") if re.match(r"^[0-9a-z]{64}$", uid)]
        answers[term] = uids

    cargo_run(["ii-build"])
    cargo_run(["config", "--set", "enable_ii", "true"])

    for term in terms:
        uids = cargo_run(["tfidf", term, "--uid-only"], stdout=True)
        uids = [uid for uid in uids.split("\n") if re.match(r"^[0-9a-z]{64}$", uid)]
        approximations[term] = uids

    zero_term, rare_term, common_term = 0, 0, 0

    for term in terms:
        answer = answers[term]
        approximation = approximations[term]

        if len(answer) >= 10:
            common_term += 1

            if len(approximation) >= 10:
                for i in range(3):
                    assert answer[i] in approximation[:10]
                    assert approximation[i] in answer[:10]

            else:
                raise AssertionError(f"len(answer)={len(answer)}, len(approximation)={len(approximation)}")

        else:
            if len(answer) == 0:
                zero_term += 1

            else:
                rare_term += 1

            if len(approximation) >= 10:
                raise AssertionError(f"len(answer)={len(answer)}, len(approximation)={len(approximation)}")

            else:
                assert set(answer) == set(approximation)

    if min(zero_term, rare_term, common_term) == 0:
        raise Exception(f"The code is fine, but the dataset is not good enough. Please add terms for better coverage. Terms and appearances: { {term: len(answers[term]) for term in answers.keys()} }")
