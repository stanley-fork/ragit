import os
import shutil
import subprocess
import sys

def init():
    from tests import goto_root
    goto_root()

    if "sample" not in os.listdir():
        os.mkdir("sample")

def load(
    repo_name: str,
    git_url: str,
    docs_at: str,
    file_ext: list[str],
    result_tmp: str,
    result_at: str,
):
    if os.path.exists(result_at):
        shutil.rmtree(result_at)

    if os.path.exists("tmp_git_dir"):
        shutil.rmtree("tmp_git_dir")

    os.mkdir("tmp_git_dir")
    os.chdir("tmp_git_dir")
    subprocess.run(["git", "clone", git_url, "--depth=1"])
    shutil.move(docs_at, "../sample")
    shutil.rmtree(repo_name)
    os.chdir("..")
    shutil.rmtree("tmp_git_dir")
    os.rename(result_tmp, result_at)
    os.chdir(result_at)
    remove_files_recursively(file_ext)

def remove_files_recursively(ext_except: list[str]):
    for file in os.listdir():
        if os.path.isdir(file):
            os.chdir(file)
            remove_files_recursively(ext_except)
            os.chdir("..")

        elif not any([file.endswith(ext) for ext in ext_except]):
            os.remove(file)

if __name__ == "__main__":
    init()

    if len(sys.argv) == 1:
        print("Please provide an argument. Valid arguments: git, postgresql, rustc, docker, kubernetes, nix")

    arg = sys.argv[1]

    if arg == "git":
        load(
            git_url = "https://github.com/git/git",
            docs_at = "./git/Documentation",
            repo_name = "git",
            file_ext = [".txt"],
            result_tmp = "./sample/Documentation",
            result_at = "./sample/git",
        )

    elif arg == "postgresql":
        load(
            git_url = "https://git.postgresql.org/git/postgresql.git",
            docs_at = "./postgresql/doc/src/sgml",
            repo_name = "postgresql",
            file_ext = [".sgml"],
            result_tmp = "./sample/sgml",
            result_at = "./sample/postgresql",
        )

    elif arg == "rustc":
        load(
            git_url = "https://github.com/rust-lang/rustc-dev-guide/",
            docs_at = "./rustc-dev-guide/src",
            repo_name = "rustc-dev-guide",
            file_ext = [".md"],
            result_tmp = "./sample/src",
            result_at = "./sample/rustc-dev-guide",
        )

    elif arg == "docker":
        load(
            git_url = "https://github.com/docker/docs/",
            docs_at = "./docs/content/manuals",
            repo_name = "docs",
            file_ext = [".md"],
            result_tmp = "./sample/manuals",
            result_at = "./sample/docker",
        )

    elif arg == "kubernetes":
        load(
            git_url = "https://github.com/kubernetes/website",
            docs_at = "./website/content/en/docs",
            repo_name = "website",
            file_ext = [".md"],
            result_tmp = "./sample/docs",
            result_at = "./sample/kubernetes",
        )

    elif arg == "tera":
        load(
            git_url = "https://github.com/Keats/tera",
            docs_at = "./tera/docs/content/docs",
            repo_name = "tera",
            file_ext = [".md"],
            result_tmp = "./sample/docs",
            result_at = "./sample/tera",
        )

    elif arg == "nix":
        load(
            git_url = "https://github.com/NixOS/nix",
            docs_at = "./nix/doc/manual/src",
            repo_name = "nix",
            file_ext = [".md"],
            result_tmp = "./sample/src",
            result_at = "./sample/os"
        )
        init()
        load(
            git_url = "https://github.com/NixOS/nixpkgs",
            docs_at = "./nixpkgs/doc",
            repo_name = "nixpkgs",
            file_ext = [".md"],
            result_tmp = "./sample/doc",
            result_at = "./sample/nixpkgs"
        )
        init()

        if os.path.exists("./sample/nix"):
            shutil.rmtree("./sample/nix")

        os.mkdir("./sample/nix")
        os.rename("./sample/os", "./sample/nix/os")
        os.rename("./sample/nixpkgs", "./sample/nix/nixpkgs")

    else:
        print("Please provide a valid argument. Valid arguments: git, postgresql, rustc, docker, kubernetes, nix")
