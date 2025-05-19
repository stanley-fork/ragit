import os
import re
import shutil
import subprocess
from utils import cargo_run, goto_root, ls_recursive, mk_and_cd_tmp_dir

def real_repos(
    # If it's not set, it writes to `/samples/{repo}`, which might overwrite an existing knowledge-base.
    # Disable this flag if you want to build a real knowledge-base, not just for test.
    tmp_dir: bool = True,

    # The test runner uses dummy model because it costs too much to build these knowledge-bases
    # with a real model. If you want to build a real knowledge-base, use a real model.
    #
    # For test purpose, `dummy` model is suffice because it can still catch all the bugs in
    # file readers.
    test_model: str = "dummy",

    # If you're building a real knowledge-base, you can specify which repo to clone and build.
    # If it's "all", it builds all.
    # If it's "nix", it builds "nix" and "nixpkgs" and merges them.
    repo: str = "all",
):
    goto_root()
    mk_and_cd_tmp_dir(
        dir_name=None if tmp_dir else "sample",
    )
    file_errors = {}

    if not os.path.exists("clone-here"):
        os.mkdir("clone-here")

    for (url, (old_name, new_name), ext) in [
        ("https://github.com/git/git", ("git", "git"), "adoc"),
        ("https://git.postgresql.org/git/postgresql.git", ("postgresql", "postgresql"), "sgml"),
        ("https://github.com/rust-lang/rustc-dev-guide/", ("rustc-dev-guide", "rustc"), "md"),
        ("https://github.com/docker/docs/", ("docs", "docker"), "md"),
        ("https://github.com/kubernetes/website", ("website", "kubernetes"), "md"),
        ("https://github.com/Keats/tera", ("tera", "tera"), "md"),
        ("https://github.com/neovim/neovim", ("neovim", "neovim"), "txt"),
        ("https://github.com/nushell/nushell.github.io", ("nushell.github.io", "nushell"), "md"),
        ("https://github.com/NixOS/nix", ("nix", "nix"), "md"),
        ("https://github.com/NixOS/nixpkgs", ("nixpkgs", "nixpkgs"), "md"),
    ]:
        if repo != "all" and repo != new_name:
            continue

        os.chdir("clone-here")

        if os.path.exists(old_name):
            shutil.rmtree(old_name)

        subprocess.run(["git", "clone", url, "--depth=1"], check=True)
        new_path = os.path.join("..", new_name)
        shutil.move(old_name, new_path)
        os.chdir(new_path)
        shutil.rmtree(".git")

        # `cargo_run` will get confused
        if os.path.exists("Cargo.toml"):
            os.remove("Cargo.toml")

        cargo_run(["init"])
        cargo_run(["config", "--set", "model", test_model])
        cargo_run(["config", "--set", "strict_file_reader", "true"])
        clean_up_repository(new_name)

        # TODO: implement `rag add **/*.md` instead of relying on shell's glob patterns
        cargo_run(["add", *ls_recursive(ext)])
        cargo_run(["build"])
        cargo_run(["check"])

        # I want to collect error messages from real world use cases, and see if it's
        # ragit's fault or their fault.
        # This process cannot be automated. It automatically collects and dumps the error
        # messages but it will not affect the result of this test.
        file_errors[new_name] = extract_error_messages(cargo_run(["build"], stdout=True))

        # For testing purposes, `strict_file_reader=true` makes more sense. But I also
        # want to use this script to create real-world knowledge-bases, and for that,
        # I have to turn off the option.
        cargo_run(["config", "--set", "strict_file_reader", "false"])
        cargo_run(["build"])
        cargo_run(["check"])

        os.chdir("..")

    if "nix" in os.listdir() and "nixpkgs" in os.listdir():
        os.mkdir("nix-real")
        os.chdir("nix-real")
        cargo_run(["init"])
        cargo_run(["merge", "../nix"])
        cargo_run(["merge", "../nixpkgs", "--prefix=nixpkgs"])
        cargo_run(["check"])

    for repo, errors in file_errors.items():
        print(f"----- {repo} ({len(errors)} errors) -----")

        for error in errors:
            print(f"    {error}")

def clean_up_repository(repo: str):
    if repo == "kubernetes":
        for lang in [
            "bn", "de", "es", "fr",
            "hi", "id", "it", "ja",
            "ko", "pl", "pt-br", "ru",
            "uk", "vi", "zh-cn",
        ]:
            shutil.rmtree(f"content/{lang}")

    if repo == "neovim":
        # contains large and meaningless text files for tests
        shutil.rmtree("test")

def extract_error_messages(stdout: str) -> list[str]:
    state = "i"
    errors = []

    for line in stdout.split("\n"):
        if state == "i":
            if re.match(r"\d+\serror(s)?", line):
                state = "e"

        elif state == "e":
            e = line.strip()

            if e != "":
                errors.append(e)

    return errors

if __name__ == "__main__":
    import sys
    repo = sys.argv[1]
    test_model = sys.argv[2]
    real_repos(
        tmp_dir=False,
        test_model=test_model,
        repo=repo,
    )
