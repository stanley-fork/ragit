import os
import re
import shutil
import subprocess
from subprocess import CalledProcessError
import time
from utils import (
    cargo_run,
    count_chunks,
    count_files,
    get_commit_hash,
    goto_root,
    ls_recursive,
    mk_and_cd_tmp_dir,
    send_message,
)

repositories = [
    {
        "git-name": "git",
        "description": "fast, scalable, distributed revision control system",
        "git-url": "https://github.com/git/git",
        "license": "GPL",
        "ragit-name": "git",
        "extensions": ["adoc"],
    }, {
        "git-name": "postgresql",
        "description": "The World's Most Advanced Open Source Relational Database",
        "git-url": "https://git.postgresql.org/git/postgresql.git",
        "license": "Postgresql License",
        "ragit-name": "postgresql",
        "extensions": ["sgml"],
    }, {
        "git-name": "rustc-dev-guide",
        "description": "A guide to how rustc works and how to contribute to it.",
        "git-url": "https://github.com/rust-lang/rustc-dev-guide/",
        "license": "Apache 2.0, MIT",
        "ragit-name": "rustc",
        "extensions": ["md"],
    }, {
        "git-name": "docs",
        "description": "Docker helps developers bring their ideas to life by conquering the complexity of app development.",
        "git-url": "https://github.com/docker/docs/",
        "license": "Apache-2.0",
        "ragit-name": "docker",
        "extensions": ["md"],
    }, {
        "git-name": "website",
        "description": "Production-Grade Container Scheduling and Management",
        "git-url": "https://github.com/kubernetes/website",
        "license": "Apache-2.0",
        "ragit-name": "kubernetes",
        "extensions": ["md"],
        "rm-r": [
            "content/bn",
            "content/de",
            "content/es",
            "content/fr",
            "content/hi",
            "content/id",
            "content/it",
            "content/ja",
            "content/ko",
            "content/pl",
            "content/pt-br",
            "content/ru",
            "content/uk",
            "content/vi",
            "content/zh-cn",
        ],
    }, {
        "git-name": "tera",
        "description": "A template engine for Rust based on Jinja2/Django",
        "git-url": "https://github.com/Keats/tera",
        "license": "MIT",
        "ragit-name": "tera",
        "extensions": ["md"],
    }, {
        "git-name": "neovim",
        "description": "Vim-fork focused on extensibility and usability",
        "git-url": "https://github.com/neovim/neovim",
        "license": "Apache-2.0",
        "ragit-name": "neovim",
        "extensions": ["txt"],

        # contains large and meaningless text files for tests
        "rm-r": ["test"],
    }, {
        "git-name": "nushell.github.io",
        "description": "A new type of shell",
        "git-url": "https://github.com/nushell/nushell.github.io",
        "license": "MIT",
        "ragit-name": "nushell",
        "extensions": ["md"],
        "rm-r": [
            "de",
            "es",
            "fr",
            "ja",
            "pt-BR",
            "ru",
            "tr",
            "zh-CN",
        ],
    }, {
        "git-name": "nix",
        "description": "Nix, the purely functional package manager",
        "git-url": "https://github.com/NixOS/nix",
        "ragit-name": "nix",
        "license": "MIT",
        "extensions": ["md"],
    }, {
        "git-name": "nixpkgs",
        "description": "Nix Packages collection & NixOS",
        "git-url": "https://github.com/NixOS/nixpkgs",
        "ragit-name": "nixpkgs",
        "license": "MIT",
        "extensions": ["md"],
    }, {
        "git-name": "zed",
        "description": "Code at the speed of thought – Zed is a high-performance, multiplayer code editor from the creators of Atom and Tree-sitter.",
        "git-url": "https://github.com/zed-industries/zed",
        "ragit-name": "zed",
        "license": "Apache-3.0, GPL",
        "extensions": ["md"],
    },
]

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

    for r in repositories:
        if repo != "all" and repo != r["ragit-name"]:
            continue

        started_at = time.time()
        send_message(f"started creating a knowledge-base of {r['ragit-name']}")
        os.chdir("clone-here")

        if os.path.exists(r["git-name"]):
            shutil.rmtree(r["git-name"])

        try:
            subprocess.run(["git", "clone", r["git-url"], "--depth=1"], check=True)

        except CalledProcessError:
            send_message(f"failed to clone {r['git-url']}")
            os.chdir("..")
            continue

        new_path = os.path.join("..", r["ragit-name"])
        shutil.move(r["git-name"], new_path)
        os.chdir(new_path)
        git_hash = get_commit_hash()
        shutil.rmtree(".git")

        # `cargo_run` will get confused
        if os.path.exists("Cargo.toml"):
            os.remove("Cargo.toml")

        if os.path.exists(".cargo"):
            shutil.rmtree(".cargo")

        cargo_run(["init"])
        cargo_run(["config", "--set", "model", test_model])
        cargo_run(["config", "--set", "strict_file_reader", "true"])

        for rm_r in r.get("rm-r", []):
            shutil.rmtree(rm_r)

        # TODO: implement `rag add **/*.md` instead of relying on shell's glob patterns
        for ext in r["extensions"]:
            cargo_run(["add", *ls_recursive(ext)])

        cargo_run(["build"], features=["full"])
        cargo_run(["check"])

        # I want to collect error messages from real world use cases, and see if it's
        # ragit's fault or their fault.
        # This process cannot be automated. It automatically collects and dumps the error
        # messages but it will not affect the result of this test.
        file_errors_ = extract_error_messages(cargo_run(["build"], features=["full"], stdout=True))
        file_errors[r["ragit-name"]] = file_errors_

        # For testing purposes, `strict_file_reader=true` makes more sense. But I also
        # want to use this script to create real-world knowledge-bases, and for that,
        # I have to turn off the option.
        cargo_run(["config", "--set", "strict_file_reader", "false"])
        cargo_run(["build"], features=["full"])
        cargo_run(["check"])

        # It's included in readme.
        cargo_run(["meta", "--set", "reproduce", how_to_reproduce(r, test_model)])
        cargo_run(["meta", "--set", "git-hash", git_hash])
        cargo_run(["meta", "--set", "git-url", r["git-url"]])
        cargo_run(["meta", "--set", "license", r["license"]])
        cargo_run(["meta", "--set", "description", r["description"]])
        cargo_run(["meta", "--set", "ai-model", test_model])
        cargo_run(["meta", "--set", "chunk-count", str(count_chunks())])
        cargo_run(["meta", "--set", "file-count", str(count_files()[2])])

        add_readme(r, test_model)
        send_message(f"finished creating a knowledge-base of {r['ragit-name']}: it took {int(time.time() - started_at)} seconds")
        send_message(f"----- {r['ragit-name']} ({len(file_errors_)} errors) -----\n" + "\n".join([f"    {e}" for e in file_errors_]))

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

def how_to_reproduce(repository, model: str) -> str:
    rm_rs = "".join([f"\nrm -r {r};" for r in repository.get("rm-r", [])])
    rag_adds = "\n".join([f"rag add **/*.{ext};" for ext in repository["extensions"]])

    return f"""
git clone {repository["git-url"]};
cd {repository["git-name"]};{rm_rs}
rag init;
# set api key of your model
rag config --set model {model};
{rag_adds}
rag build;
"""

def add_readme(repository, model: str):
    from datetime import datetime
    git_hash = cargo_run(["meta", "--get", "git-hash"], stdout=True).strip()
    chunk_count = cargo_run(["meta", "--get", "chunk-count"], stdout=True).strip()
    file_count = cargo_run(["meta", "--get", "file-count"], stdout=True).strip()
    reproduce = cargo_run(["meta", "--get", "reproduce"], stdout=True).strip()

    readme = f"""# {repository["ragit-name"]}

{repository["description"]}

This knowledge-base was auto-generated by script. It's built by {model} at {datetime.now()}.

- source: [{repository["git-url"]}]({repository["git-url"]})
- license: {repository["license"]}
- git hash: {git_hash}
- chunks: {chunk_count}
- files: {file_count}

## How to clone

`rag clone http://ragit.baehyunsol.com/sample/{repository["ragit-name"]}`

## How to reproduce

```sh
{reproduce}
```
"""
    cargo_run(["meta", "--set", "readme", readme])
    return readme

if __name__ == "__main__":
    import sys
    repo = sys.argv[1]
    test_model = sys.argv[2]
    real_repos(
        tmp_dir=False,
        test_model=test_model,
        repo=repo,
    )
