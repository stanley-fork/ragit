import os
import re
import subprocess
from typing import Optional
from utils import goto_root

def cargo_tests():
    goto_root()
    errors = run_cargo_test(
        location="core",
        additional_actions=[
            ["cargo", "test", "--release", "--features=csv,korean,pdf,svg"],
            ["cargo", "doc", "--features=csv,korean,pdf,svg"],
        ],
    )
    os.chdir("crates")

    for crate in ["api", "cli", "fs", "ignore", "korean", "pdl", "server"]:
        os.chdir(crate)
        errors += run_cargo_test(
            location=crate,

            # build artifacts take more than 20GiB
            additional_actions=[["cargo", "clean"]],
        )
        os.chdir("..")

    if len(errors) > 0:
        raise Exception("\n\n".join(errors))

# returns error messages, if exists
def run_cargo_test(
    location: str,
    additional_actions: Optional[list[list[str]]] = None,
) -> list[str]:
    errors = []
    actions = [
        ["cargo", "test"],
        ["cargo", "test", "--release"],
        ["cargo", "doc"],
    ]
    actions += additional_actions or []

    for action in actions:
        print(f"running `{' '.join(action)}` at `{location}`")
        result = subprocess.run(action, capture_output=True, text=True)

        if result.returncode != 0 or has_warning(result.stderr):
            errors.append(f"""
#####################
### path: command ###
{os.getcwd()}: {' '.join(action)}

### status_code ###
{result.returncode}

### stdout ###
{result.stdout}

### stderr ###
{clean_cargo_output(result.stderr)}
""")

    return errors

def has_warning(stderr: str) -> bool:
    warnings = re.search(r"warning\:.+generated\s(\d+)\swarning", stderr)
    return warnings is not None and int(warnings.group(1)) > 0

def clean_cargo_output(stdout: str) -> str:
    i_dont_want_to_see_these = lambda line: re.match(r"^\s*Compiling\s.+v\d.+", line) or\
        re.match(r"^\s*Documenting\s.+v\d.+", line) or\
        re.match(r"^\s*Checking\s.+v\d.+", line)

    return "\n".join([line for line in stdout.split("\n") if not i_dont_want_to_see_these(line)])
