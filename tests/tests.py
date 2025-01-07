from add_and_rm import add_and_rm
from cargo_tests import cargo_tests
from cat_file import cat_file
from cli import cli
from clone import clone
from empty import empty
from end_to_end import end_to_end
from external_bases import external_bases
from ii import ii
from images import images
from images2 import images2
from ls import ls
from many_chunks import many_chunks
from markdown_reader import markdown_reader
from merge import merge
from migrate import migrate
from prompts import prompts
from ragit_api import ragit_api
from recover import recover
from subdir import subdir
from tfidf import tfidf

import os
import sys
from utils import clean, goto_root

def get_commit_hash():
    try:
        import subprocess
        return subprocess.run(["git", "rev-parse", "HEAD"], capture_output=True, text=True, check=True).stdout.strip()

    except Exception as e:
        return f"cannot get commit_hash: {e}"

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

    recover                     run `recover` test
                                It checks whether 1) `rag check` fails on a broken
                                knowledge-base and 2) `rag check --recover` can
                                fix a broken knowledge-base.

    clone                       run `clone` test
                                It creates a knowledge-base, pushes, clones and checks it.

    cli                         run `cli` test
                                It tests whether cli parser can parse the arguments correctly.

    migrate                     run `migrate` test
                                It checks out git to v 0.1.1, creates a knowledge-base,
                                and run `migrate` until the knowledge-base is migrated to
                                the newest version.
                                Since it runs `git checkout`, it may mess up your working
                                tree. If you have uncommitted changes, this test will fail
                                and does not mess up your working tree.

    many_chunks                 run `many_chunks` test
                                It creates a lot of small files and see if ragit can
                                handle the files correctly. It also tests interrupting
                                `rag build`.

    ls                          run `ls` test
                                It runs `ls-files`, `ls-chunks`, and `tfidf` with bunch
                                of different options.

    empty [model=dummy]         run `empty` test
                                It sees if ragit can handle an empty file correctly.

    ii                          run `ii` test
                                It creates an inverted index and test it.

    cat-file                    run `cat-file` test

    images                      run `images` test
                                It creates a markdown file with images and check
                                whether the markdown reader can parse the file
                                correctly.

    images2 [model]             run `images2` test
                                It tests whether models can generate image-description
                                files correctly.
                                NOTE: It uses the vision capability of the model.
                                      Make sure that the model has one.

    markdown_reader             run `markdown_reader` test
                                I have found many bugs in `markdown_reader_v0`. The bugs
                                are reproduced in this test. If you find a new one, please
                                add that to this test.

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

    ragit_api [model]           run `ragit_api` test
                                It asks "what's your name" to the model. It returns OK
                                if the api call was successful. It doesn't care about the
                                content of the model's response.

    cargo_tests                 run `cargo test` on all the crates

    all                         run all tests
                                It dumps the test result to `tests/results.json`.
"""

if __name__ == "__main__":
    no_clean = "--no-clean" in sys.argv
    args = [arg for arg in sys.argv if arg != "--no-clean"]
    command = args[1] if len(args) > 1 else None
    test_model = args[2] if len(args) > 2 else None

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

        elif command == "recover":
            recover()

        elif command == "clone":
            clone()

        elif command == "cli":
            cli()

        elif command == "migrate":
            migrate()

        elif command == "many_chunks":
            many_chunks()

        elif command == "ls":
            ls()

        elif command == "empty":
            test_model = test_model or "dummy"
            empty(test_model)

        elif command == "ii":
            ii()

        elif command == "cat_file":
            cat_file()

        elif command == "images":
            images()

        elif command == "images2":
            if test_model is None or test_model == "dummy":
                print("Please specify which model to run the tests with. You cannot run this test with a dummy model.")
                sys.exit(1)

            images2(test_model=test_model)

        elif command == "markdown_reader":
            markdown_reader()

        elif command == "prompts":
            test_model = test_model or "dummy"
            prompts(test_model=test_model)

        elif command == "subdir":
            subdir()

        elif command == "tfidf":
            tfidf()

        elif command == "ragit_api":
            if test_model is None:
                print("Please specify which model to run the tests with.")
                sys.exit(1)

            ragit_api(test_model=test_model)

        elif command == "cargo_tests":
            cargo_tests()

        elif command == "all":
            from datetime import datetime
            import json
            import time
            import traceback

            tests = [
                ("cargo_tests", cargo_tests),
                ("add_and_rm", add_and_rm),
                ("recover", recover),
                ("clone", clone),
                ("cli", cli),
                ("many_chunks", many_chunks),
                ("ls", ls),
                ("ii", ii),
                ("cat_file", cat_file),
                ("images", images),
                ("markdown_reader", markdown_reader),
                ("subdir", subdir),
                ("tfidf", tfidf),
                ("merge", merge),
                ("external_bases", external_bases),
                ("end_to_end dummy", lambda: end_to_end(test_model="dummy")),
                ("end_to_end gpt-4o-mini", lambda: end_to_end(test_model="gpt-4o-mini")),
                ("prompts dummy", lambda: prompts(test_model="dummy")),
                ("prompts gpt-4o-mini", lambda: prompts(test_model="gpt-4o-mini")),
                ("prompts claude-3.5-sonnet", lambda: prompts(test_model="claude-3.5-sonnet")),
                ("empty dummy", lambda: empty(test_model="dummy")),
                ("empty gpt-4o-mini", lambda: empty(test_model="gpt-4o-mini")),
                ("images2 gpt-4o-mini", lambda: images2(test_model="gpt-4o-mini")),

                # TODO: replace it with haiku when haiku's vision becomes available
                ("images2 claude-3.5-sonnet", lambda: images2(test_model="claude-3.5-sonnet")),

                # NOTE: dummy, openai and anthropic models are already tested above
                ("ragit_api llama3.2-11b-groq", lambda: ragit_api(test_model="llama3.2-11b-groq")),
                ("ragit_api command-r", lambda: ragit_api(test_model="command-r")),
                ("ragit_api phi-3-14b-ollama", lambda: ragit_api(test_model="phi-3-14b-ollama")),
                ("migrate", migrate),
            ]
            started_at = datetime.now()
            has_error = False
            result = {
                "meta": {
                    "complete": False,
                    "started_at": str(started_at),
                    "commit": get_commit_hash(),
                    "platform": get_platform_info(),
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
                f.write(json.dumps(result, indent=4))

            for name, test in tests:
                try:
                    start = time.time()
                    test()

                except Exception as e:
                    has_error = True
                    result["tests"][name] = {
                        "pass": False,
                        "error": str(e) + "\n" + traceback.format_exc(),
                        "elapsed_ms": int((time.time() - start) * 1000),
                    }
                    result["result"]["fail"] += 1

                else:
                    result["tests"][name] = {
                        "pass": True,
                        "error": None,
                        "elapsed_ms": int((time.time() - start) * 1000),
                    }
                    result["result"]["pass"] += 1

                finally:
                    result["result"]["complete"] += 1
                    result["result"]["remaining"] -= 1

                    if not no_clean:
                        clean()

                    goto_root()
                    os.chdir("tests")

                    with open("result.json", "w") as f:
                        f.write(json.dumps(result, indent=4))

            ended_at = datetime.now()
            result["meta"]["ended_at"] = str(ended_at)
            result["meta"]["elapsed_ms"] = (ended_at - started_at).seconds * 1000 + (ended_at - started_at).microseconds // 1000
            result["meta"]["complete"] = True
            goto_root()
            os.chdir("tests")
            result = json.dumps(result, indent=4)
            print(result)

            with open("result.json", "w") as f:
                f.write(result)

            if has_error:
                sys.exit(1)

        else:
            print(help_message)

    finally:
        if not no_clean:
            clean()
