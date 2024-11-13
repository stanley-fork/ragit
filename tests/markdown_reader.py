from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, write_string

# fixed by 8e06403f
sample1 = '''
this is a sentence. `this is an unterminated
code span`.
'''

# fixed by bf1906cb
sample2 = "\n".join([
    "a" * 3500,
    *(["aaaa"] * 200),
])

# fixed by bf1906cb
sample3 = "\n".join([
    "a" * 6000,
    "aa",
    "aa",
])

def markdown_reader():
    goto_root()
    mk_and_cd_tmp_dir()

    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])

    write_string("sample1.md", sample1)
    cargo_run(["add", "sample1.md"])
    cargo_run(["build"], timeout=20.0)
    cargo_run(["check"])

    write_string("sample2.md", sample2)
    cargo_run(["config", "--set", "chunk_size", "4000"])
    cargo_run(["config", "--set", "slide_len", "1000"])
    cargo_run(["add", "sample2.md"])
    cargo_run(["build"], timeout=20.0)
    cargo_run(["check"])

    write_string("sample3.md", sample3)
    cargo_run(["config", "--set", "chunk_size", "4000"])
    cargo_run(["config", "--set", "slide_len", "1000"])
    cargo_run(["add", "sample3.md"])
    cargo_run(["build"], timeout=20.0)
    cargo_run(["check"])
