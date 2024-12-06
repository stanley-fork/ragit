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

# not fixed yet
sample4 = """
2. some title
  - some sentence
  - another sentence
    - `<|media(PATH/TO/YOUR/MEDIA/FILE)|>`
    - `<|raw_media(png:BASE64_VALUE_OF_YOUR_MEDIA_FILE)|>`. For now, it supports `png`, `jpeg`, `gif` and `webp`.

You'll find pdl files in 2 places: your local ragit repo and ragit's git repo.

1. If you have initialized a ragit repo, you'll find pdl files in `./.ragit/prompts`. Modify the files and run `rag build` or `rag query` to see how LLM behaves differently. Make sure to `rag config --set dump_log true` so that you can see the conversations.
2. You can also find `prompts/` in ragit's git repo. This is the default value for prompts. If your local tests on your new prompts are satisfiable, please commit the new prompts.
"""

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

    write_string("sample4.md", sample4)
    cargo_run(["config", "--set", "chunk_size", "64"])
    cargo_run(["config", "--set", "slide_len", "16"])
    cargo_run(["add", "sample4.md"])
    cargo_run(["build"], timeout=20.0)
    cargo_run(["check"])
