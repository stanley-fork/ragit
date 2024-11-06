from add_and_rm import add_and_rm
from auto_recover import auto_recover
from cargo_tests import cargo_tests
from end_to_end import end_to_end
from external_bases import external_bases
from images import images
from images2 import images2
import os
import sys
from tfidf import tfidf
from utils import clean, goto_root

help_message = """
Commands
    end_to_end [model=dummy]    run `end_to_end` test

    external_bases              run `external_bases` test

    add_and_rm                  run `add_and_rm` test

    auto_recover                run `auto_recover` test

    images                      run `images` test

    images2 [model]             run `images2` test

    tfidf                       run `tfidf` test

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

        elif command == "auto_recover":
            auto_recover()

        elif command == "images":
            images()

        elif command == "images2":
            if test_model is None or test_model == "dummy":
                print("Please specify which model to run the tests with. You cannot run this test with a dummy model.")
                sys.exit(1)

            images2(test_model=test_model)

        elif command == "tfidf":
            tfidf()

        elif command == "cargo_tests":
            cargo_tests()

        elif command == "all":
            import json
            import time
            import traceback

            has_error = False
            results = {}
            tests = [
                ("end_to_end-dummy", lambda: end_to_end(test_model="dummy")),
                ("external_bases", external_bases),
                ("add_and_rm", add_and_rm),
                ("auto_recover", auto_recover),
                ("images", images),
                ("cargo_tests", cargo_tests),
            ]

            for model in [
                "gpt-4o-mini",
                "claude-3-5-sonnet",
            ]:
                tests.append((f"end_to_end {model}", lambda: end_to_end(test_model=model)))
                tests.append((f"images2 {model}", lambda: images2(test_model=model)))

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
