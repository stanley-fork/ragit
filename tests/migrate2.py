from migrate import checkout
import os
import shutil
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir

def migrate2():
    goto_root()
    mk_and_cd_tmp_dir()
    errors = []
    curr_version = "0.3.4"

    # ragit 0.2.1 or older versions are not compatible with the current implementation of ragit-server
    for old_version in ["0.3.0", "0.3.1", "0.3.2", "0.3.3"]:
        checkout(old_version)

        for url in [
            "http://ragit.baehyunsol.com/sample/ragit",
            "http://ragit.baehyunsol.com/sample/git",
            "http://ragit.baehyunsol.com/sample/rustc",
        ]:
            clone_result = cargo_run(["clone", url], check=False, output_schema=["returncode", "stdout", "stderr"])

            if clone_result["returncode"] != 0:
                errors.append(f"""
#####################
### path: command ###
version: {old_version}
{os.getcwd()}: rag clone {url}

### returncode ###

{clone_result["returncode"]}

### stdout ###

{clone_result["stdout"]}

### stderr ###

{clone_result["stderr"]}"""
                )
                continue

        checkout(curr_version)

        for base in ["ragit", "git", "rustc"]:
            if os.path.exists(base):
                os.chdir(base)
                cargo_run(["migrate"])
                cargo_run(["check"])
                os.chdir("..")
                shutil.rmtree(base)

    if errors != []:
        raise Exception("\n\n".join(errors))
