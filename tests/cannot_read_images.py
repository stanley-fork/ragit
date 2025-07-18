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

    finally:
        if server_process is not None:
            server_process.kill()
