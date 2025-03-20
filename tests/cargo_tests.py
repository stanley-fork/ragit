import os
import subprocess
from subprocess import CalledProcessError
from utils import goto_root

# returns error messages, if exists
def run_cargo_test() -> list[str]:
    errors = []

    for action in [
        ["cargo", "test"],
        ["cargo", "test", "--release"],
        ["cargo", "doc"],
    ]:
        result = subprocess.run(action, capture_output=True, text=True)

        if result.returncode != 0:
            errors.append(f"""
#####################
### path: command ###
{os.getcwd()}: {' '.join(action)}

### status_code ###
{result.returncode}

### stdout ###
{result.stdout}

### stderr ###
{result.stderr}
""")

    return errors

def cargo_tests():
    goto_root()
    errors = run_cargo_test()
    os.chdir("crates")

    for crate in ["api", "fs", "ignore", "korean", "pdl", "server"]:
        os.chdir(crate)
        errors += run_cargo_test()
        os.chdir("..")

    if len(errors) > 0:
        raise Exception("\n\n".join(errors))
