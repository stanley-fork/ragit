import os
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

# '-C' option
# The option has to be as similar to git as possible.
def gh_issue_20():
    goto_root()
    mk_and_cd_tmp_dir()

    # There are 3 `sample.md` files.
    # <tmp>/sample.md
    # <tmp>/base1/sample.md
    # <tmp>/base2/sample.md
    write_string("sample.md", "This is a sample: abcde")

    os.mkdir("base1")
    os.mkdir("base2")

    cargo_run(["-C", "base1", "init"])
    write_string("base1/sample.md", "This is a sample: fghij")

    cargo_run(["-C", "base2", "init"])
    write_string("base2/sample.md", "This is a sample: klmno")

    os.chdir("base1")
    cargo_run(["-C", "../base2", "add", "sample.md"])
    os.chdir("../base2")
    cargo_run(["-C", "../base1", "add", "sample.md"])
    os.chdir("..")

    for base, magic_word in [
        ("base1", "fghij"),
        ("base2", "klmno"),
    ]:
        os.chdir(base)
        cargo_run(["config", "--set", "model", "dummy"])
        cargo_run(["build"])
        cargo_run(["check"])
        assert magic_word in cargo_run(["cat-file", "sample.md"], stdout=True)
        os.chdir("..")

    cargo_run(["-C", "base1", "rm", "sample.md"])
    cargo_run(["-C", "base1", "add", "."])
    cargo_run(["-C", "base1", "build"])
    assert "fghij" in cargo_run(["-C", "base1", "cat-file", "sample.md"], stdout=True)
