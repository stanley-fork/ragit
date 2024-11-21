from add_and_rm import add_and_rm
from cargo_tests import cargo_tests
from clone import clone
from end_to_end import end_to_end
from external_bases import external_bases
from images import images
from images2 import images2
from many_chunks import many_chunks
from markdown_reader import markdown_reader
from migrate import migrate
from ragit_api import ragit_api
from recover import recover
from tfidf import tfidf

import os
import sys
from utils import clean, goto_root

help_message = """
Commands
    end_to_end [model=dummy]    run `end_to_end` test
                                It simulates a basic workflow of ragit: init,
                                add, build and query. It runs on a real dataset:
                                the documents of ragit.

    external_bases              run `external_bases` test
                                It creates bunch of knowledge-bases and run
                                `rag ext` on them. It also checks whether `rag tfidf`
                                can successfully retrieve a chunk from multiple knowledge-bases.

    add_and_rm                  run `add_and_rm` test
                                It runs tons of `rag add` and `rag rm` with
                                different options.

    recover                     run `recover` test
                                It checks whether 1) `rag check` fails on a broken
                                knowledge-base and 2) `rag check --recover` can
                                fix a broken knowledge-base.

    clone                       run `clone` test
                                It creates a knowledge-base, pushes, clones and checks it.

    migrate                     run `migrate` test (TODO)

    many_chunks                 run `many_chunks` test
                                It creates a lot of small files and see if ragit can
                                handle the files correctly. It also tests interrupting
                                `rag build`.

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
    command = sys.argv[1] if len(sys.argv) > 1 else None
    test_model = sys.argv[2] if len(sys.argv) > 2 else None

    try:
        if command == "end_to_end":
            test_model = test_model or "dummy"
            end_to_end(test_model=test_model)

        elif command == "external_bases":
            external_bases()

        elif command == "add_and_rm":
            add_and_rm()

        elif command == "recover":
            recover()

        elif command == "clone":
            clone()

        elif command == "migrate":
            migrate()

        elif command == "many_chunks":
            many_chunks()

        elif command == "images":
            images()

        elif command == "images2":
            if test_model is None or test_model == "dummy":
                print("Please specify which model to run the tests with. You cannot run this test with a dummy model.")
                sys.exit(1)

            images2(test_model=test_model)

        elif command == "markdown_reader":
            markdown_reader()

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
            import json
            import time
            import traceback

            has_error = False
            results = {}
            tests = [
                ("external_bases", external_bases),
                ("add_and_rm", add_and_rm),
                ("recover", recover),
                ("clone", clone),
                ("migrate", migrate),
                ("many_chunks", many_chunks),
                ("images", images),
                ("markdown_reader", markdown_reader),
                ("cargo_tests", cargo_tests),
                ("tfidf", tfidf),
                ("end_to_end dummy", lambda: end_to_end(test_model="dummy")),
                ("end_to_end gpt-4o-mini", lambda: end_to_end(test_model="gpt-4o-mini")),
                ("images2 gpt-4o-mini", lambda: images2(test_model="gpt-4o-mini")),

                # TODO: replace it with haiku when haiku's vision becomes available
                ("images2 claude-3.5-sonnet", lambda: images2(test_model="claude-3.5-sonnet")),

                # NOTE: dummy, openai and anthropic models are already tested above
                ("ragit_api llama3.2-11b-groq", lambda: ragit_api(test_model="llama3.2-11b-groq")),
                ("ragit_api command-r", lambda: ragit_api(test_model="command-r")),
                ("ragit_api phi-3-14b-ollama", lambda: ragit_api(test_model="phi-3-14b-ollama")),
            ]

            for name, test in tests:
                try:
                    start = time.time()
                    test()

                except Exception as e:
                    has_error = True
                    end = time.time()
                    results[name] = {
                        "pass": False,
                        "error": str(e) + "\n" + traceback.format_exc(),
                        "elapsed_ms": int((end - start) * 1000),
                    }

                else:
                    end = time.time()
                    results[name] = {
                        "pass": True,
                        "error": None,
                        "elapsed_ms": int((end - start) * 1000),
                    }

                finally:
                    clean()

            goto_root()
            os.chdir("tests")
            result = json.dumps(results, indent=4)
            print(result)

            with open("results.json", "w") as f:
                f.write(result)

            if has_error:
                sys.exit(1)

        else:
            print(help_message)

    finally:
        clean()
