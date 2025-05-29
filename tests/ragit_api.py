from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

def ragit_api(test_model: str):
    goto_root()
    mk_and_cd_tmp_dir()
    write_string("hello_world.pdl", "\n<|user|>\n\nWhat's your name?\n")
    cargo_run(["init"])
    cargo_run(["pdl", "hello_world.pdl", "--model", test_model])
