import os
from subprocess import TimeoutExpired
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir

def clone2():
    goto_root()
    mk_and_cd_tmp_dir()
    errors = []

    for repo_name, url in [
        ("git", "http://ragit.baehyunsol.com/sample/git"),
        ("ragit", "http://ragit.baehyunsol.com/sample/ragit"),
        ("rustc", "http://ragit.baehyunsol.com/sample/rustc"),
    ]:
        try:
            clone_result = cargo_run(["clone", url], timeout=300.0, check=False, output_schema=["returncode", "stdout", "stderr"])

            if clone_result["returncode"] != 0:
                errors.append(f"""
#####################
### path: command ###
{os.getcwd()}: rag clone {url}

### returncode ###

{clone_result["returncode"]}

### stdout ###

{clone_result["stdout"]}

### stderr ###

{clone_result["stderr"]}"""
                )
                continue

            os.chdir(repo_name)
            check_result = cargo_run(["check"], check=False, output_schema=["returncode", "stdout", "stderr"])
            os.chdir("..")

            if check_result["returncode"] != 0:
                errors.append(f"""
#####################
### path: command ###
{os.getcwd()}: rag check

### returncode ###

{check_result["returncode"]}

### stdout ###

{check_result["stdout"]}

### stderr ###

{check_result["stderr"]}"""
                )
                continue

        except TimeoutExpired:
            errors.append(f"""
#####################
### path: command ###
{os.getcwd()}: rag clone {url}

timeout"""
            )

    if errors != []:
        raise Exception("\n\n".join(errors))
