from migrate import checkout
import os
import shutil
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir

def migrate2():
    goto_root()
    mk_and_cd_tmp_dir()
    curr_version = "0.4.2"

    # ragit 0.2.1 or older versions are not compatible with the current implementation of ragit-server
    for old_version in ["0.3.0", "0.3.5", "0.4.0"]:
        checkout(old_version)

        for base, url in [
            ("ragit", "https://ragit.baehyunsol.com/sample/ragit"),
            ("git", "https://ragit.baehyunsol.com/sample/git"),
            ("rustc", "https://ragit.baehyunsol.com/sample/rustc"),
        ]:
            # ragit 0.3.x are not compatible with the current implementation of ragit-server, but I want to test them anyway
            if old_version.startswith("0.3."):
                checkout(curr_version)
                cargo_run(["clone", url])
                os.chdir(base)

                # 0.4.x and newer supports gemini models, but 0.3.x does not
                # so I'll just ignore the models
                cargo_run(["model", "--remove", "--all"])

                # `rag clone` is not compatible, but `rag archive-create` must be available in 0.3.x
                checkout(old_version)
                cargo_run(["archive-create", "-o", "../ar"])
                os.chdir("..")
                shutil.rmtree(base)
                cargo_run(["archive-extract", "ar", "-o", base])
                os.remove("ar")

            else:
                clone_result = cargo_run(["clone", url])

        checkout(curr_version)

        for base in ["ragit", "git", "rustc"]:
            if os.path.exists(base):
                os.chdir(base)
                cargo_run(["migrate"])
                cargo_run(["check"])
                os.chdir("..")
                shutil.rmtree(base)
