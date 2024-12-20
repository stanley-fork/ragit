# The goal of this test is to cover all the pdl files in `prompts/` directory.
# It has to cover different paths of each prompt. For example, `summarize.pdl` behaves
# differently when there's `{{previous_summary}}` and when there's not.

import shutil
from utils import cargo_run, count_chunks, goto_root, mk_and_cd_tmp_dir, write_string

def prompts(test_model: str):
    goto_root()
    mk_and_cd_tmp_dir()
    shutil.copyfile("../tests/images/hello_world.webp", "sample.webp")
    write_string("sample.md", "This image says hello world: ![image](sample.webp)")
    shutil.copyfile("../src/main.rs", "main.rs")

    with open("main.rs", "r") as f:
        main_rs = f.read()

    if len(main_rs) < 10000:
        raise Exception("This test case requires a text file that has at least 10000 characters. Please choose another file.")

    if len(main_rs) > 10000:
        write_string("main.rs", main_rs[:10000])

    cargo_run(["init"])
    cargo_run(["config", "--set", "strict_file_reader", "true"])
    cargo_run(["config", "--set", "chunk_size", "1000"])
    cargo_run(["config", "--set", "slide_len", "200"])
    cargo_run(["config", "--set", "model", test_model])
    cargo_run(["config", "--set", "dump_log", "true"])
    cargo_run(["add", "sample.md", "main.rs"])
    cargo_run(["check"])

    # `summarize.pdl`, `describe_image.pdl`
    cargo_run(["build"])
    cargo_run(["check"])

    chunks = count_chunks()

    if chunks not in range(12, 16):
        raise Exception(f"Expected 12~15 chunks, got {chunks}.")

    # `extract_keyword.pdl`, `rerank_summary.pdl`, `answer_query.pdl`
    cargo_run(["config", "--set", "max_titles", "5"])
    cargo_run(["config", "--set", "max_summaries", "4"])
    cargo_run(["config", "--set", "max_retrieval", "2"])
    cargo_run(["query", "You're looking at a source code of a command line utility. What does the main function do?"])

    # `rerank_title.pdl`, `answer_query.pdl`
    cargo_run(["config", "--set", "max_titles", "32"])
    cargo_run(["config", "--set", "max_summaries", "4"])
    cargo_run(["config", "--set", "max_retrieval", "2"])
    cargo_run(["query", "You're looking at a source code of a command line utility. What does the main function do?"])

    # TODO: `multi_turn.pdl`

    # `raw.pdl`
    cargo_run(["remove", "sample.md"])
    cargo_run(["remove", "main.rs"])
    assert count_chunks() == 0
    cargo_run(["query", "You're looking at a source code of a command line utility. What does the main function do?"])
