import os
import subprocess
from utils import goto_root

goto_root()
subprocess.run(["cargo", "clean"], check=True)

os.chdir("crates")

for crate in os.listdir():
    os.chdir(crate)
    subprocess.run(["cargo", "clean"], check=True)
    os.chdir("..")
