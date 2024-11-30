from random import random, seed as rand_seed
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, rand_word, write_string

def cat_file():
    rand_seed(0)
    goto_root()
    mk_and_cd_tmp_dir()
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["config", "--set", "chunk_size", "500"])
    cargo_run(["config", "--set", "slide_len", "150"])
    files = {}

    for i in range(10):
        # for now, ragit chooses file reader based on file extensions
        for ext in ["md", "txt"]:
            file_name = f"{i}.{ext}"
            file_content = "".join([rand_word() + (" " if random() < 0.8 else "\n") for _ in range(i * 20)])
            write_string(file_name, file_content)
            files[file_name] = file_content
            cargo_run(["add", file_name])

    cargo_run(["build"])

    for file_name, file_content in files.items():
        assert cargo_run(["cat-file", file_name], stdout=True).strip() == file_content.strip()
