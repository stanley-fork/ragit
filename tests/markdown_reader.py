from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, write_string

# fix 8e06403f
sample1 = '''
this is a sentence. `this is an unterminated
code span`.
'''

def markdown_reader():
    goto_root()
    mk_and_cd_tmp_dir()

    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])

    write_string("sample1.md", sample1)
    cargo_run(["add", "sample1.md"])
    cargo_run(["build"], timeout=20.0)
    cargo_run(["check"])
