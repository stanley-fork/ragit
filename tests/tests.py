from add_and_rm import add_and_rm
from add_and_rm2 import add_and_rm2
from archive import archive
from audit import audit
from cargo_tests import cargo_tests
from cargo_features import cargo_features
from cat_file import cat_file
from clean_up_erroneous_chunk import clean_up_erroneous_chunk
from cli import cli
from clone import clone
from clone_empty import clone_empty
from config import config
from csv_reader import csv_reader
from empty import empty
from end_to_end import end_to_end
from external_bases import external_bases
from extract_keywords import extract_keywords
from generous_file_reader import generous_file_reader
from gh_issue_20 import gh_issue_20
from ignore import ignore
from ii import ii
from images import images
from images2 import images2
from images3 import images3
from korean import korean
from logs import logs
from ls import ls
from many_chunks import many_chunks
from many_jobs import many_jobs
from markdown_reader import markdown_reader
from merge import merge
from meta import meta
from migrate import migrate
from migrate2 import migrate2
from migrate3 import migrate3
from models_init import models_init, test_home_config_override
from orphan_process import orphan_process
from outside import outside
from pdf import pdf
from pdl import pdl
from prompts import prompts
from pull import pull
from query_options import query_options
from query_with_schema import query_with_schema
from ragit_api import ragit_api
from real_repos import real_repos
from real_repos_regression import real_repos_regression
from recover import recover
from server import server
from server_chat import server_chat
from server_permission import server_permission
from subdir import subdir
from svg import svg
from symlink import symlink
from tfidf import tfidf
from web_images import web_images
from write_lock import write_lock

from datetime import datetime
import os
from random import seed as rand_seed
import sys
from utils import (
    clean,
    clean_test_output,
    get_commit_hash,
    get_ragit_version,
    goto_root,
)

def get_platform_info() -> dict[str, str]:
    result = {}

    try:
        import subprocess
        result["cargo_version"] = subprocess.run(["cargo", "version"], capture_output=True, text=True, check=True).stdout.strip()

    except Exception as e:
        result["cargo_version"] = f"cannot get cargo_version: {e}"

    try:
        result["rustc_version"] = subprocess.run(["rustc", "--version"], capture_output=True, text=True, check=True).stdout.strip()

    except Exception as e:
        result["rustc_version"] = f"cannot get rustc_version: {e}"

    try:
        import platform
        result["python_version"] = platform.python_version()

    except Exception as e:
        result["python_version"] = f"cannot get python_version: {e}"

    try:
        result["platform"] = platform.platform()

    except Exception as e:
        result["platform"] = f"cannot get platform: {e}"

    return result

help_message = """
Commands
    end_to_end [model=dummy]    run `end_to_end` test
                                It simulates a basic workflow of ragit: init,
                                add, build and query. It runs on a real dataset:
                                the documents of ragit.

    external_bases              run `external_bases` test
                                It creates bunch of knowledge-bases and run
                                `rag merge` on them. It also checks whether `rag tfidf`
                                can successfully retrieve a chunk from multiple knowledge-bases.

    merge                       run `merge` test
                                It's like `external_bases` test, but with `--prefix` option.

    add_and_rm                  run `add_and_rm` test
                                It runs tons of `rag add` and `rag rm` with
                                different options.

    add_and_rm2                 run `add_and_rm2` test
                                Like `add_and_rm`, but it's more focused on `rag rm`.

    ignore                      run `ignore` test
                                It tests whether `rag add` respects `.ragignore` or `.gitignore`.

    archive                     run `archive` test
                                It runs `archive-create` and `archive-extract` and check
                                if the extracted knowledge-base is identical to the original
                                one.

    recover                     run `recover` test
                                It checks whether 1) `rag check` fails on a broken
                                knowledge-base and 2) `rag check --recover` can
                                fix a broken knowledge-base.

    clone                       run `clone` test
                                It creates a knowledge-base, pushes, clones and checks it.
                                It runs a local `ragit-server` in this repository.

    clone_empty                 run `clone_empty` test
                                It creates an empty repository in ragit-server, clones the
                                repository (which is not an error), adds some chunks to it,
                                and pushes it back to the server.

    pull                        run `pull` test
                                It creates a repository, pushes and pulls the repository and see
                                if it works.

    server                      run `server` test
                                It tests endpoints related to a repository. It first pushes a
                                repository and fetches data (chunks, images, files, ...) from
                                the server.

    server_chat [model]         run `server_chat` test
                                It tests chat-related endpoints of ragit-server.

    server_permission           run `server_permission` test
                                It creates users and repositories with different permissions
                                and sends requests with/without api keys.

    query_options [model]       run `query_options` test
                                It tests various option flags of `rag query`.

    query_with_schema [model]   run `query_with_schema` test
                                It tests `--schema` flag of `rag query`.

    cli                         run `cli` test
                                It tests whether cli parser can parse the arguments correctly.

    outside                     run `outside`
                                It tests whether ragit can successfully reject files outside
                                a knowledge-base.

    migrate                     run `migrate` test
                                It checks out git to v 0.1.1, creates a knowledge-base,
                                and run `migrate` until the knowledge-base is migrated to
                                the newest version.
                                Since it runs `git checkout`, it may mess up your working
                                tree. If you have uncommitted changes, this test will fail
                                and does not mess up your working tree.

    migrate2                    run `migrate2` test
                                Like `migrate`, but clones knowledge-bases from web instead
                                of creating a mock knowledge-base.

    migrate3                    run `migrate3` test
                                It creates knowledge-bases with different versions of ragit.
                                Then it makes sure that the versions can clone/push to the
                                latest version of ragit-server.

    config                      run `config` test
                                I have added new configs to ragit 0.3.5. And I want to see
                                if it's compatible with older versions.

    gh_issue_20                 run `gh_issue_20` test
                                https://github.com/baehyunsol/ragit/issues/20
                                It tests `-C` option.

    many_chunks                 run `many_chunks` test
                                It creates a lot of small files and see if ragit can
                                handle the files correctly. It also tests interrupting
                                `rag build`.

    many_jobs [model=dummy] [jobs=999]
                                run `many_jobs` test
                                `rag build` by default runs with many processes, and a
                                multi-process program may introduce many unexpected bugs.
                                It runs `rag build` with many processes and see if it works.
                                You'd better run it on a machine with many cores.

    ls                          run `ls` test
                                It runs `ls-files`, `ls-chunks`, and `tfidf` with bunch
                                of different options.

    logs [model]                run `logs` test
                                It checks if `rag config --set dump_log true` and
                                `rag gc --logs` work correctly.

    meta                        run `meta` test
                                It runs `rag meta`-family commands and see if it works.

    empty [model=dummy]         run `empty` test
                                It sees if ragit can handle an empty file correctly.

    symlink                     run `symlink` test
                                It tests whether ragit can handle symlinks correctly
                                without falling into infinite loops.

    ii                          run `ii` test
                                It creates an inverted index and test it.

    cat_file                    run `cat_file` test

    generous_file_reader        run `generous_file_reader` test
                                If some files are broken, ragit is supposed to
                                skip the broken files and continue processing the
                                valid files.

    clean_up_erroneous_chunk    run `clean_up_erroneous_chunk` test
                                It's an edge case in `generous_file_reader`.

    audit [model]               run `audit` test

    images                      run `images` test
                                It creates a markdown file with images and check
                                whether the markdown reader can parse the file
                                correctly.

    images2 [model]             run `images2` test
                                It tests whether models can generate image-description
                                files correctly.
                                NOTE: It uses the vision capability of the model.
                                      Make sure that the model has one.

    images3 [model]             run `images3` test
                                Other tests test images in markdown files, but they
                                don't test image file readers. It does.

    pdf [model]                 run `pdf` test
                                It tests the pdf reader.
                                You have to use a vision language model!

    pdl [model]                 run `pdl` test
                                It tests `rag pdl` command.

    svg [model]                 run `svg` test
                                It tests the svg reader.
                                You have to use a vision language model!

    web_images [model]          run `web_images` test
                                It tests whether ragit can fetch images from web.

    extract_keywords [model]    run `extract_keywords` test
                                It tests whether `rag extract-keywords` command works.

    orphan_process              run `orphan_process` test
                                It reproduces gh issue #9.
                                https://github.com/baehyunsol/ragit/issues/9

    write_lock                  run `write_lock` test
                                It reproduces gh issue #8.
                                https://github.com/baehyunsol/ragit/issues/9

    markdown_reader             run `markdown_reader` test
                                I have found many bugs in `markdown_reader_v0`. The bugs
                                are reproduced in this test. If you find a new one, please
                                add that to this test.

    csv_reader                  run `csv_reader` test

    real_repos [repo=all]       run `real_repos` test
                                Clone real git repos from the web and build knowledge-base
                                on the repos. It's to test file readers, not LLMs.

    real_repos_regression       run `real_repos_regression` test
                                I ran `python3 tests.py real_repos` and was surprised to see
                                it throwing so many errors. Many of them were ragit's fault. So
                                I created this test, which tries to reproduce all the errors found
                                in the `real_repos` test.

    prompts [model=dummy]       run `prompts` test
                                It's the smallest set of commands that parses and executes
                                all the `.pdl` files in `prompts/` directory.

    subdir                      run `subdir` test
                                It checks whether `ragit` is smart enough to find `.ragit/`
                                in any directory.

    tfidf                       run `tfidf` test
                                It creates bunch of lorem-ipsum files and see if
                                `rag tfidf` can retrieve files correctly. It also tests
                                tfidf searches on cjk strings.

    korean                      run `korean` test
                                It runs ragit with/without "korean" feature and makes sure
                                that the tokenizer behaves differently.

    ragit_api [model]           run `ragit_api` test
                                It asks "what's your name" to the model. It returns OK
                                if the api call was successful. It doesn't care about the
                                content of the model's response.

    cargo_tests                 run `cargo test` on all the crates

    cargo_features              run `cargo_features` test
                                Ragit has many cargo features. This test compiles
                                ragit with all the possible combinations of features
                                and makes sure that they all compile.

    models_init                 run `models_init` test
                                It tests the initialization of models.json and
                                model selection in api.json.

    all                         run all tests
                                It dumps the test result to `tests/results.json`.
"""

if __name__ == "__main__":
    no_clean = "--no-clean" in sys.argv
    args = [arg for arg in sys.argv if arg != "--no-clean"]
    seed = [arg for arg in args if arg.startswith("--seed=")]

    if len(seed) > 0:
        args = [arg for arg in args if arg not in seed]
        seed = int(seed[0].split("=")[1])

    else:
        now = datetime.now()
        seed = int(f"{now.year:04}{now.month:02}{now.day:02}{now.hour:02}{now.minute:02}{now.second:02}")

    command = args[1] if len(args) > 1 else None
    test_model = args[2] if len(args) > 2 else None
    rand_seed(seed)

    try:
        if command == "end_to_end":
            test_model = test_model or "dummy"
            end_to_end(test_model=test_model)

        elif command == "external_bases":
            external_bases()

        elif command == "merge":
            merge()

        elif command == "add_and_rm":
            add_and_rm()

        elif command == "add_and_rm2":
            add_and_rm2()

        elif command == "ignore":
            ignore()

        elif command == "recover":
            recover()

        elif command == "clone":
            clone()

        elif command == "clone_empty":
            clone_empty()

        elif command == "pull":
            pull()

        elif command == "server":
            server()

        elif command == "server_chat":
            if test_model is None or test_model == "dummy":
                print("Please specify which model to run the tests with. You cannot run this test with a dummy model.")
                sys.exit(1)

            server_chat(test_model=test_model)

        elif command == "server_permission":
            server_permission()

        elif command == "query_options":
            if test_model is None or test_model == "dummy":
                print("Please specify which model to run the tests with. You cannot run this test with a dummy model.")
                sys.exit(1)

            query_options(test_model=test_model)

        elif command == "query_with_schema":
            if test_model is None or test_model == "dummy":
                print("Please specify which model to run the tests with. You cannot run this test with a dummy model.")
                sys.exit(1)

            query_with_schema(test_model=test_model)

        elif command == "cli":
            cli()

        elif command == "outside":
            outside()

        elif command == "audit":
            if test_model is None or test_model == "dummy":
                print("Please specify which model to run the tests with. You cannot run this test with a dummy model.")
                sys.exit(1)

            audit(test_model=test_model)

        elif command == "migrate":
            migrate()

        elif command == "migrate2":
            migrate2()

        elif command == "migrate3":
            migrate3()

        elif command == "config":
            config()

        elif command == "gh_issue_20":
            gh_issue_20()

        elif command == "archive":
            archive()

        elif command == "many_chunks":
            many_chunks()

        elif command == "many_jobs":
            jobs = args[3] if len(args) > 3 else 999
            test_model = test_model if test_model else "dummy"
            many_jobs(test_model=test_model, jobs=jobs)

        elif command == "ls":
            ls()

        elif command == "logs":
            if test_model is None or test_model == "dummy":
                print("Please specify which model to run the tests with. You cannot run this test with a dummy model.")
                sys.exit(1)

            logs(test_model=test_model)

        elif command == "meta":
            meta()

        elif command == "symlink":
            symlink()

        elif command == "empty":
            test_model = test_model or "dummy"
            empty(test_model)

        elif command == "ii":
            ii()

        elif command == "cat_file":
            cat_file()

        elif command == "generous_file_reader":
            generous_file_reader()

        elif command == "clean_up_erroneous_chunk":
            clean_up_erroneous_chunk()

        elif command == "images":
            images()

        elif command == "images2":
            if test_model is None or test_model == "dummy":
                print("Please specify which model to run the tests with. You cannot run this test with a dummy model.")
                sys.exit(1)

            images2(test_model=test_model)

        elif command == "images3":
            if test_model is None or test_model == "dummy":
                print("Please specify which model to run the tests with. You cannot run this test with a dummy model.")
                sys.exit(1)

            images3(test_model=test_model)

        elif command == "pdf":
            if test_model is None or test_model == "dummy":
                print("Please specify which model to run the tests with. You cannot run this test with a dummy model.")
                sys.exit(1)

            pdf(test_model=test_model)

        elif command == "pdl":
            if test_model is None or test_model == "dummy":
                print("Please specify which model to run the tests with. You cannot run this test with a dummy model.")
                sys.exit(1)

            pdl(test_model=test_model)

        elif command == "svg":
            if test_model is None or test_model == "dummy":
                print("Please specify which model to run the tests with. You cannot run this test with a dummy model.")
                sys.exit(1)

            svg(test_model=test_model)

        elif command == "web_images":
            if test_model is None or test_model == "dummy":
                print("Please specify which model to run the tests with. You cannot run this test with a dummy model.")
                sys.exit(1)

            web_images(test_model=test_model)

        elif command == "extract_keywords":
            if test_model is None:
                print("Please specify which model to run the tests with.")
                sys.exit(1)

            extract_keywords(test_model=test_model)

        elif command == "orphan_process":
            if test_model is None or test_model == "dummy":
                print("Please specify which model to run the tests with. You cannot run this test with a dummy model.")
                sys.exit(1)

            orphan_process(test_model=test_model)

        elif command == "write_lock":
            if test_model is None or test_model == "dummy":
                print("Please specify which model to run the tests with. You cannot run this test with a dummy model.")
                sys.exit(1)

            write_lock(test_model=test_model)

        elif command == "markdown_reader":
            markdown_reader()

        elif command == "csv_reader":
            csv_reader()

        elif command == "real_repos":
            repo = "all" if len(args) < 3 else args[2]
            real_repos(repo=repo)

        elif command == "real_repos_regression":
            real_repos_regression()

        elif command == "prompts":
            test_model = test_model or "dummy"
            prompts(test_model=test_model)

        elif command == "subdir":
            subdir()

        elif command == "tfidf":
            tfidf()

        elif command == "korean":
            korean()

        elif command == "ragit_api":
            if test_model is None:
                print("Please specify which model to run the tests with.")
                sys.exit(1)

            ragit_api(test_model=test_model)

        elif command == "cargo_tests":
            cargo_tests()

        elif command == "cargo_features":
            cargo_features()
            
        elif command == "models_init":
            models_init()
            test_home_config_override()

        elif command == "all":
            import json
            import time
            import traceback

            tests = [
                ("cargo_tests", cargo_tests),
                ("cargo_features", cargo_features),
                ("add_and_rm", add_and_rm),
                ("add_and_rm2", add_and_rm2),
                ("ignore", ignore),
                ("recover", recover),
                ("clone", clone),
                ("clone_empty", clone_empty),
                ("pull", pull),
                ("server", server),
                ("server_permission", server_permission),
                ("cli", cli),
                ("outside", outside),
                ("archive", archive),
                ("many_chunks", many_chunks),
                ("many_jobs", many_jobs),
                ("ls", ls),
                ("meta", meta),
                ("symlink", symlink),
                ("gh_issue_20", gh_issue_20),
                ("ii", ii),
                ("cat_file", cat_file),
                ("generous_file_reader", generous_file_reader),
                ("clean_up_erroneous_chunk", clean_up_erroneous_chunk),
                ("images", images),
                ("markdown_reader", markdown_reader),
                ("csv_reader", csv_reader),
                ("real_repos", real_repos),
                ("real_repos_regression", real_repos_regression),
                ("subdir", subdir),
                ("tfidf", tfidf),
                ("korean", korean),
                ("merge", merge),
                ("external_bases", external_bases),
                ("end_to_end dummy", lambda: end_to_end(test_model="dummy")),
                ("end_to_end llama3.3-70b", lambda: end_to_end(test_model="llama3.3-70b")),
                ("audit llama3.3-70b", lambda: audit(test_model="llama3.3-70b")),
                ("logs llama3.3-70b", lambda: logs(test_model="llama3.3-70b")),
                ("prompts dummy", lambda: prompts(test_model="dummy")),
                ("prompts gpt-4o-mini", lambda: prompts(test_model="gpt-4o-mini")),
                ("prompts gemini-2.0-flash", lambda: prompts(test_model="gemini-2.0-flash")),
                ("prompts claude-3.5-sonnet", lambda: prompts(test_model="claude-3.5-sonnet")),
                ("empty dummy", lambda: empty(test_model="dummy")),
                ("empty llama3.3-70b", lambda: empty(test_model="llama3.3-70b")),
                ("server_chat llama3.3-70b", lambda: server_chat(test_model="llama3.3-70b")),
                ("server_chat gemini-2.0-flash", lambda: server_chat(test_model="gemini-2.0-flash")),
                ("images2 gpt-4o-mini", lambda: images2(test_model="gpt-4o-mini")),
                ("images3 gpt-4o-mini", lambda: images3(test_model="gpt-4o-mini")),
                ("pdl gpt-4o-mini", lambda: pdl(test_model="gpt-4o-mini")),
                ("pdf gpt-4o-mini", lambda: pdf(test_model="gpt-4o-mini")),
                ("svg gpt-4o-mini", lambda: svg(test_model="gpt-4o-mini")),
                ("web_images gpt-4o-mini", lambda: web_images(test_model="gpt-4o-mini")),

                # TODO: replace it with haiku when haiku's vision becomes available
                ("images2 claude-3.5-sonnet", lambda: images2(test_model="claude-3.5-sonnet")),

                ("extract_keywords dummy", lambda: extract_keywords(test_model="dummy")),
                ("extract_keywords gpt-4o-mini", lambda: extract_keywords(test_model="gpt-4o-mini")),
                ("orphan_process llama3.3-70b", lambda: orphan_process(test_model="llama3.3-70b")),
                ("write_lock llama3.3-70b", lambda: write_lock(test_model="llama3.3-70b")),
                ("ragit_api command-r", lambda: ragit_api(test_model="command-r")),
                ("query_options llama3.3-70b", lambda: query_options(test_model="llama3.3-70b")),
                ("query_with_schema llama3.3-70b", lambda: query_with_schema(test_model="llama3.3-70b")),
                ("models_init", models_init),
                ("test_home_config_override", test_home_config_override),
                ("config", config),
                ("migrate", migrate),
                ("migrate2", migrate2),
                ("migrate3", migrate3),
            ]
            started_at = datetime.now()
            has_error = False
            result = {
                "meta": {
                    "complete": False,
                    "started_at": str(started_at),
                    "commit": get_commit_hash(),
                    "platform": get_platform_info(),
                    "ragit_version": get_ragit_version(),
                    "rand_seed": seed,
                },
                "tests": {},
                "result": {
                    "total": len(tests),
                    "complete": 0,
                    "pass": 0,
                    "fail": 0,
                    "remaining": len(tests),
                },
            }

            with open("result.json", "w") as f:
                f.write(json.dumps(result, indent=4, ensure_ascii=True))

            for seq, (name, test) in enumerate(tests):
                print(f"running `{name}`...")

                try:
                    start = time.time()
                    rand_seed(seed)
                    test()

                except Exception as e:
                    has_error = True
                    result["tests"][name] = {
                        "seq": seq,
                        "pass": False,
                        "error": clean_test_output(str(e) + "\n" + traceback.format_exc()),
                        "elapsed_ms": int((time.time() - start) * 1000),
                    }
                    result["result"]["fail"] += 1

                else:
                    result["tests"][name] = {
                        "seq": seq,
                        "pass": True,
                        "error": None,
                        "elapsed_ms": int((time.time() - start) * 1000),
                    }
                    result["result"]["pass"] += 1

                finally:
                    result["result"]["complete"] += 1
                    result["result"]["remaining"] -= 1

                    if not no_clean:
                        try:
                            clean()

                        # `clean()` may die. For example, some tests may spawn a process and dies while
                        # its children are alive. The children are still writing something to the tmp dir
                        # and it would mess up `shutil.rmtree()`.
                        except Exception as e:
                            result["tests"][name]["cleanup_error"] = str(e) + "\n" + traceback.format_exc()

                    goto_root()
                    os.chdir("tests")

                    with open("result.json", "w") as f:
                        f.write(json.dumps(result, indent=4, ensure_ascii=True))

            ended_at = datetime.now()
            result["meta"]["ended_at"] = str(ended_at)
            result["meta"]["elapsed_ms"] = (ended_at - started_at).seconds * 1000 + (ended_at - started_at).microseconds // 1000
            result["meta"]["complete"] = True
            goto_root()
            os.chdir("tests")
            result = json.dumps(result, indent=4, ensure_ascii=True)
            print(result)

            with open("result.json", "w") as f:
                f.write(result)

            if has_error:
                sys.exit(1)

        else:
            print("invalid command:", command)
            print(help_message)

    finally:
        if not no_clean:
            clean()
