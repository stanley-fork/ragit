from fake_llm_server import host_fake_llm_server
import json
import shutil
import time
from utils import (
    cargo_run,
    count_files,
    count_images,
    goto_root,
    mk_and_cd_tmp_dir,
    rand_word,
    read_string,
    write_string,
)

def cannot_read_images():
    goto_root()
    server_process = host_fake_llm_server()

    try:
        mk_and_cd_tmp_dir()
        shutil.copyfile("../tests/images/red.jpg", "red.jpg")
        write_string("image.md", "here's an image: ![](red.jpg)")
        write_string("text.md", "there's no image here")

        cargo_run(["init"])
        models = json.loads(read_string(".ragit/models.json"))
        erroneous_model = {
            "name": "text-only-model",
            "api_provider": "openai",
            "api_url": "http://127.0.0.1:11435/api/chat",
            "api_name": "text-only",
            "can_read_images": False,
            "input_price": 0.0,
            "output_price": 0.0,
        }
        models.append(erroneous_model)
        write_string(".ragit/models.json", json.dumps(models))
        cargo_run(["config", "--set", "model", "text-only-model"])
        cargo_run(["config", "--set", "sleep_between_retries", "99999999999"])

        cargo_run(["add", "image.md", "text.md"])
        started_at = time.time()
        build_result = cargo_run(["build"], stdout=True)
        elapsed = time.time() - started_at

        # `text.md` must succeed immediately and `image.md`
        # must fail immediately (nothing to do with `sleep_between_retries`)
        assert elapsed < 1

        # `text.md` -> processed
        # `image.md` -> staged
        assert count_files() == (2, 1, 1)  # (total, staged, processed)
        assert "CannotReadImage" in build_result
        assert count_images() == 0

        # I found another bug!
        # 1. Process a file that has an image. It has more than 1 chunks. The first chunk
        #    doesn't have an image, but a later chunk has.
        # 2. The file is processed with a text-only model. The model successfully creates
        #    the first chunk, but fails at a later chunk.
        # 3. It has to remove the chunks and tfidf indexes from step 2, but it only removes
        #    the chunks.
        # 4. It later messes up with `rag tfidf`, or any other command that reads the tfidf
        #    index.

        magic_word = rand_word()
        scientific_word = "abcdef"  # "scientific" is the opposite of "magic" HAHAHA
        document = [
            magic_word,
            *([scientific_word] * 200),
            magic_word,
            "here's an image: ![](red.jpg)",
            magic_word,
        ]
        document = "\n".join(document)
        write_string("bug.md", document)
        cargo_run(["config", "--set", "chunk_size", "1000"])
        cargo_run(["config", "--set", "slide_len", "200"])
        cargo_run(["config", "--set", "image_size", "300"])

        cargo_run(["rm", "--all"])
        cargo_run(["add", "bug.md", "image.md", "text.md"])

        # llama3.3 is a text-only model.
        # I can't use dummy-ish model here because a dummy-ish
        # model always create chunks with the same uid.
        # When llama3.3 is deprecated, use another text-only
        # model. If there's no text-only model at all, this
        # test can be deprecated.
        cargo_run(["config", "--set", "model", "llama3.3"])
        cargo_run(["build"])
        cargo_run(["check"])
        assert count_files() == (3, 2, 1)

        cargo_run(["config", "--set", "model", "dummy"])
        cargo_run(["build"])
        cargo_run(["check"])
        assert count_files() == (3, 0, 3)

        cargo_run(["tfidf", magic_word])

    finally:
        if server_process is not None:
            server_process.kill()
