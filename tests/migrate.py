from images import sample_markdown
from markdown_reader import sample1, sample2, sample3
import shutil
import subprocess
from subprocess import CalledProcessError
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, write_string

# TODO: any better way?
def checkout(version: str):
    commit_hashes = {
        "0.1.1": "a168d13af967",
        "0.2.0": "d6030d107105",
    }

    try:
        subprocess.run(["git", "checkout", commit_hashes[version]], check=True)

    except CalledProcessError:
        raise Exception(f"Cannot git-checkout to ragit version {version}, please commit your changes before running the test.")

def migrate():
    goto_root()
    checkout("0.1.1")
    mk_and_cd_tmp_dir()

    # step 1. create a mock knowledge-base
    write_string("sample0.md", "Hi! My name is Baehyunsol.")
    write_string("sample1.md", sample1)
    write_string("sample2.md", sample2)
    write_string("sample3.md", sample3)

    # NOTE: v 0.1.1 itself has a bug and I cannot fix that
    # shutil.copyfile("../tests/images/empty.png", "sample2.png")
    # shutil.copyfile("../tests/images/empty.jpg", "sample5.jpg")
    # shutil.copyfile("../tests/images/empty.webp", "sample6.webp")
    # write_string("sample4.md", sample_markdown)

    # step 2. init and build rag index
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["add", "sample0.md", "sample1.md", "sample2.md", "sample3.md"])
    cargo_run(["build"])
    cargo_run(["check"])

    checkout("0.2.0")
    assert cargo_run(["check"], check=False) != 0
    cargo_run(["migrate"])
    cargo_run(["check"])
    assert "sample0.md" in cargo_run(["tfidf", "baehyunsol"], stdout=True)
