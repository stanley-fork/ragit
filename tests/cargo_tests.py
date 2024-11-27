import os
import subprocess
from utils import goto_root

def cargo_tests():
    goto_root()
    subprocess.run(["cargo", "test"], check=True)
    subprocess.run(["cargo", "test", "--release"], check=True)
    os.chdir("crates")

    for crate in ["api", "fs", "korean", "server"]:
        os.chdir(crate)
        subprocess.run(["cargo", "test"], check=True)
        subprocess.run(["cargo", "test", "--release"], check=True)
        os.chdir("..")
