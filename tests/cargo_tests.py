import os
import subprocess
from subprocess import CalledProcessError
from utils import goto_root

def cargo_tests():
    goto_root()
    subprocess.run(["cargo", "test"], check=True)
    subprocess.run(["cargo", "test", "--release"], check=True)
    os.chdir("crates")

    for crate in ["api", "fs", "korean", "pdl", "server"]:
        os.chdir(crate)

        try:
            subprocess.run(["cargo", "test"], check=True)
            subprocess.run(["cargo", "test", "--release"], check=True)

        except CalledProcessError as e:
            raise Exception(f"Error in {crate}: {e}")

        os.chdir("..")
