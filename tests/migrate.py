from markdown_reader import sample1, sample2, sample3
import shutil
import subprocess
from subprocess import CalledProcessError
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, write_string

# TODO: any better way?
def checkout(version: str):
    commit_hashes = {
        "0.1.1": "a168d13af967",
        "0.2.0": "d14773e55cce5",
        "0.2.1": "281a98f41f37",
        "0.3.0": "205f212adbd7",
    }

    try:
        subprocess.run(["git", "checkout", commit_hashes[version]], check=True)

    except CalledProcessError:
        raise Exception(f"Cannot git-checkout to ragit version {version}, please commit your changes before running the test.")

# make sure that there's no `.ragit/` directory
# if so, remove it
def init_knowledge_base():
    write_string("sample0.md", "Hi! My name is Baehyunsol.")
    write_string("sample1.md", sample1)
    write_string("sample2.md", sample2)
    write_string("sample3.md", sample3)

    shutil.copyfile("../tests/images/empty.png", "sample2.png")
    shutil.copyfile("../tests/images/empty.jpg", "sample5.jpg")
    shutil.copyfile("../tests/images/empty.webp", "sample6.webp")
    write_string("sample4.md", "image1: ![sample2](sample2.png)\nimage2: ![sample5](sample5.jpg)\nimage3: ![sample6](sample6.webp)")

    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["add", "sample0.md", "sample1.md", "sample2.md", "sample3.md", "sample4.md"])
    cargo_run(["build"])
    cargo_run(["query", "Who is baehyunsol?"])
    cargo_run(["check"])

def migrate():
    goto_root()
    mk_and_cd_tmp_dir()

    # NOTE: I found that 0.1.1 -> 0.2.0 -> 0.2.1 is broken because the implementation
    #       of 0.2.0 is broken. I just fixed 0.1.1 -> 0.2.1 migration.
    # # step 1: init knowledge-base in version 0.1.1
    # checkout("0.1.1")
    # init_knowledge_base()

    # # step 2: 0.1.1 and 0.2.0 are not compatible
    # checkout("0.2.0")
    # assert cargo_run(["check"], check=False) != 0
    # cargo_run(["migrate"])
    # cargo_run(["check"])
    # assert "sample0.md" in cargo_run(["tfidf", "baehyunsol"], stdout=True)

    # # step 3: 0.2.0 and 0.2.1 are compatible
    # checkout("0.2.1")
    # cargo_run(["check"])
    # assert "sample0.md" in cargo_run(["tfidf", "baehyunsol"], stdout=True)

    # # step 3.1: `rag migrate` is no-op
    # cargo_run(["migrate"])
    # cargo_run(["check"])

    # step 4: init knowledge-base in version 0.2.0
    checkout("0.2.0")
    init_knowledge_base()

    # step 5: 0.2.0 and 0.2.1 are compatible
    checkout("0.2.1")
    cargo_run(["check"])
    cargo_run(["migrate"])
    cargo_run(["check"])

    # step 6: 0.2.1 to 0.3.0
    checkout("0.3.0")
    cargo_run(["check"])
    cargo_run(["migrate"])
    cargo_run(["check"])

    # step 7: direct migrate from 0.1.1 to 0.3.0
    checkout("0.1.1")
    shutil.rmtree(".ragit")
    init_knowledge_base()

    checkout('0.3.0')
    assert cargo_run(["check"], check=False) != 0
    cargo_run(["migrate"])
    cargo_run(["check"])
    assert "sample0.md" in cargo_run(["tfidf", "baehyunsol"], stdout=True)
