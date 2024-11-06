import os
import subprocess
from utils import goto_root, write_string

def ragit_api(test_model: str):
    goto_root()
    os.chdir("crates/api")
    write_string("hello_world.pdl", "\n<|user|>\n\nWhat's your name?\n")
    subprocess.run(["cargo", "run", "--release", '--', '--model', test_model, '--input', 'hello_world.pdl'], check=True)
