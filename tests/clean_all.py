import os
import subprocess
from utils import goto_root

goto_root()
subprocess.run(["cargo", "clean"], check=True)

if os.path.exists("Cargo.lock"):
    os.remove("Cargo.lock")

for path in [
    "crates/api",
    "crates/cli",
    "crates/fs",
    "crates/ignore",
    "crates/korean",
    "crates/pdl",
    "ragithub/frontend",
    "ragithub/backend",
]:
    os.chdir(path)
    subprocess.run(["cargo", "clean"], check=True)

    if os.path.exists("Cargo.lock"):
        os.remove("Cargo.lock")

    os.chdir("../..")
