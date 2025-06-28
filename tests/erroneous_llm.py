from fake_llm_server import host_fake_llm_server
import json
from utils import (
    cargo_run,
    count_files,
    goto_root,
    mk_and_cd_tmp_dir,
    rand_word,
    read_string,
    write_string,
)

def erroneous_llm():
    goto_root()
    server_process = host_fake_llm_server()

    try:
        mk_and_cd_tmp_dir()
        cargo_run(["init"])

        cargo_run(["config", "--set", "dump_log", "true"])
        files = []

        for i in range(20):
            write_string(f"{i}.md", " ".join([rand_word() for _ in range(20)]))
            files.append(f"{i}.md")

        # TODO: implement `rag add *.md`
        cargo_run(["add", *files])

        # Test 1: building a knowledge-base with a rate-limited model
        models = json.loads(read_string(".ragit/models.json"))
        erroneous_model = {
            "name": "rate-limited-model",
            "api_provider": "openai",
            "api_url": "http://127.0.0.1:11435/api/chat",
            "api_name": "rate-limit-20",
            "can_read_images": True,
            "input_price": 0.0,
            "output_price": 0.0,
        }
        models.append(erroneous_model)
        write_string(".ragit/models.json", json.dumps(models))
        cargo_run(["config", "--set", "model", "rate-limited-model"])

        # If the rate limit is 20 messages per minute, it can build the
        # knowledge-base because it retries a few times after sleeping.
        # It would take significantly longer, though.
        cargo_run(["build"])
        cargo_run(["check"])
        assert count_files() == (20, 0, 20)

        # Let's prepare for the next test
        cargo_run(["remove", "--all"])
        cargo_run(["add", *files])
        assert count_files() == (20, 20, 0)

        # Test 2: building a knowledge-base when the server frequently fails (500 error)
        models = json.loads(read_string(".ragit/models.json"))
        erroneous_model["name"] = "unstable-model"
        erroneous_model["api_name"] = "fail-5"
        models.append(erroneous_model)
        write_string(".ragit/models.json", json.dumps(models))
        cargo_run(["config", "--set", "model", "unstable-model"])

        # In this case it might not able to completely build the knowledge-base.
        # But there must be a progress.
        cargo_run(["build"])
        cargo_run(["check"])
        total, _, processed = count_files()
        assert total == 20 and processed > 0

    finally:
        if server_process is not None:
            server_process.kill()
