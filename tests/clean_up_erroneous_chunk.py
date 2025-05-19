from utils import (
    cargo_run,
    count_files,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

# I found this case while running `python3 tests.py real_repos docker`. The problem is
# 1. When you process a file with multiple chunks
# 2. and the first chunk of the file is okay but there's a problem with another chunk in the file,
# 3. ragit has to remove all the chunks from the file and continue processing the other files.
# 4. But ragit fails to remove some chunks of the file.
def clean_up_erroneous_chunk():
    goto_root()
    mk_and_cd_tmp_dir()
    cargo_run(["init"])
    write_string("sample.md", "abcdefg" * 100 + "![](https://invalid/url.png)")
    cargo_run(["add", "sample.md"])
    cargo_run(["config", "--set", "strict_file_reader", "true"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["config", "--set", "chunk_size", "500"])
    cargo_run(["config", "--set", "slide_len", "100"])

    cargo_run(["build"])
    cargo_run(["check"])

    assert count_files() == (1, 1, 0)  # (total, staged, processed)
