import json
import shutil
from utils import (
    cargo_run,
    count_files,
    goto_root,
    mk_and_cd_tmp_dir,
)

def pdf(test_model: str):
    # The dummy model can build a knowledge-base,
    # but the knowledge-base will be filled with
    # empty chunks.
    for test_model in ["dummy", test_model]:
        goto_root()
        mk_and_cd_tmp_dir()
        cargo_run(["init"])
        shutil.copyfile("../tests/pdfs/landscape.pdf", "landscape.pdf")
        shutil.copyfile("../tests/pdfs/portrait.pdf", "portrait.pdf")

        cargo_run(["add", "landscape.pdf", "portrait.pdf"])
        cargo_run(["config", "--set", "model", test_model])
        cargo_run(["config", "--set", "dump_log", "true"])
        cargo_run(["config", "--set", "summary_after_build", "false"])

        # make sure that it doesn't work without "pdf" feature
        assert "FeatureNotEnabled" in cargo_run(["build"], features=[], stdout=True)
        assert count_files() == (2, 2, 0)  # (total, staged, processed)

        cargo_run(["build"], features=["pdf"])
        cargo_run(["check"])

        pdfs = [
            {
                "name": "landscape.pdf",
                "pages": 5,
                "keywords": ["inline", "assembly"],
                "query": ("How do I embed inline assembly to my Rust program?", "asm!"),
            }, {
                # TODO: "끝말잇기" is not a good topic for ragit. It's so bad that
                #       I set `"query": None`. I have to find more technical topic.
                "name": "portrait.pdf",
                "pages": 1,
                "keywords": ["끝말잇기"],

                # Sadly, most models are not smart enough to retrieve `portrait.pdf`
                "query": None,
            },
        ]

        for pdf in pdfs:
            chunks = json.loads(cargo_run(["ls-chunks", pdf["name"], "--json"], stdout=True))
            page_nos = [chunk["source"]["page"] for chunk in chunks]
            page_nos.sort()
            assert page_nos == list(range(1, pdf["pages"] + 1))

            # Ragit's pdf reader relies on vision capability of models.
            # Hence, the dummy model cannot read pdf files.
            if test_model != "dummy":
                for keyword in pdf["keywords"]:
                    search_result = json.loads(cargo_run(["tfidf", keyword, "--json"], stdout=True))
                    assert any([pdf["name"] in r["source"] for r in search_result])

                if pdf["query"] is not None:
                    query, answer = pdf["query"]
                    assert answer in cargo_run(["query", query], stdout=True)
