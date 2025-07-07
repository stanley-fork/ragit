from fake_llm_server import host_fake_llm_server
import json
import os
import shutil
from utils import (
    cargo_run,
    deepcopy,
    goto_root,
    mk_and_cd_tmp_dir,
    read_string,
    write_string,
)

def pdl_escape():
    goto_root()
    server_process = host_fake_llm_server()
    mk_and_cd_tmp_dir()

    try:
        write_string("filter.pdl", """
<|user|>

{% for content in contents %}
{{content}}
{% endfor %}
""")
        write_string("no-filter.pdl", """
<|user|>

{% for content in contents %}
{{content|safe}}
{% endfor %}
""")
        context = {
            "contents": [
                "<|schema|>",
                "This is an invalid <|schema|>",
                "<|media|>",
                "<|media(empty.png)|>",
            ],
        }
        write_string("pdl-tokens.json", json.dumps(context))

        # special tokens are all escaped, so there's no problem at all!
        cargo_run(["pdl", "filter.pdl", "--model=dummy", "--log=logs", "--context=pdl-tokens.json"])
        log_file = [f"logs/{file}" for file in os.listdir("logs") if file.endswith(".pdl")]
        assert len(log_file) == 1
        log_file = log_file[0]
        log_file = read_string(log_file)

        assert "<|schema|>" in log_file and "<|media|>" in log_file

        # Fails because there's `<|schema|>`.
        assert cargo_run(["pdl", "no-filter.pdl", "--model=dummy", "--context=pdl-tokens.json"], check=False) != 0

        context2 = { "contents": ["<|media(empty.png)|>"] }
        write_string("media-pdl-token.json", json.dumps(context2))

        # Fails because there's no `empty.png`.
        assert cargo_run(["pdl", "no-filter.pdl", "--model=dummy", "--context=media-pdl-token.json"], check=False) != 0

        # Succeeds with `empty.png`.
        shutil.copyfile("../tests/images/empty.png", "empty.png")
        cargo_run(["pdl", "no-filter.pdl", "--model=dummy", "--context=media-pdl-token.json"])

        cargo_run(["init"])
        cargo_run(["config", "--set", "model", "dummy"])
        cargo_run(["config", "--set", "dump_log", "true"])

        # If a file contains pdl tokens, they have to be properly escaped.
        write_string("no-image.md", "<|schema|> <- this should be escaped!\n\n<|media(empty.png)|> <- this should be escaped!")
        cargo_run(["add", "no-image.md"])
        cargo_run(["build"])

        # The AI must see the string "<|media(empty.png)|>"
        log_file = [f".ragit/logs/{file}" for file in os.listdir(".ragit/logs") if file.endswith(".pdl")]
        assert len(log_file) == 1
        log_file = log_file[0]
        log_file = read_string(log_file)
        assert "<|schema|>" in log_file and "<|media(empty.png)|>" in log_file
        cargo_run(["gc", "--logs"])

        chunk = json.loads(cargo_run(["ls-chunks", "--json"], stdout=True))[0]
        assert len(chunk["images"]) == 0
        cargo_run(["rm", "--all"])

        # If there's an image in markdown, ragit will turn that into a pdl token: `<|raw_media(png:...)|>`
        write_string("image.md", "<|schema|> <- this should be escaped!\n\n![image](empty.png) <- this should be rendered to an image!")
        cargo_run(["add", "image.md"])
        cargo_run(["build"])

        # The AI must see the image.
        log_file = [f".ragit/logs/{file}" for file in os.listdir(".ragit/logs") if file.endswith(".pdl")]

        # There are more than 1 pdl files, but they all have the image.
        # assert len(log_file) == 1

        log_file = log_file[0]
        log_file = read_string(log_file)
        assert "<|raw_media(png:" in log_file
        cargo_run(["gc", "--logs"])

        chunk = json.loads(cargo_run(["ls-chunks", "--json"], stdout=True))[0]
        assert len(chunk["images"]) == 1

        # TODO: impl `rag add *.md`
        cargo_run(["add", "image.md", "no-image.md"])
        cargo_run(["build"])
        cargo_run(["query", "Since we're using a dummy model, we cannot expect it to answer a query. We just want to make sure that there's no pdl error while handling the contexts."])
        cargo_run(["query", "--agent", "Since we're using a dummy model, we cannot expect it to answer a query. We just want to make sure that there's no pdl error while handling the contexts."])

        # I want to make sure that `rag query` works even though the AI response contains string "<|schema|>"
        # let's create a dummy model who always replies with the same response: "<|schema|>"
        models = json.loads(read_string(".ragit/models.json"))
        dummy_model = {
            "name": "dummy1",
            "api_provider": "openai",
            "api_url": "http://127.0.0.1:11435/api/chat",
            "api_name": "dummy-" + "".join([f"{ord(c):02x}" for c in "<|schema|>"]),
            "can_read_images": True,
            "input_price": 0.0,
            "output_price": 0.0,
        }
        models.append(dummy_model)
        write_string(".ragit/models.json", json.dumps(models))
        cargo_run(["config", "--set", "model", "dummy1"])
        assert "<|schema|>" in cargo_run(["query", "Since we're using a dummy model, we cannot expect it to answer a query. We just want to make sure that there's no pdl error while handling the contexts."], stdout=True)
        assert "<|schema|>" in cargo_run(["query", "--agent", "Since we're using a dummy model, we cannot expect it to answer a query. We just want to make sure that there's no pdl error while handling the contexts."], stdout=True)

        # I want to make sure that it doesn't filter out html tags.
        repeat_after_me_model = deepcopy(dummy_model)
        repeat_after_me_model["name"] = "repeat-after-me"
        repeat_after_me_model["api_name"] = "repeat-after-me"
        models.append(repeat_after_me_model)
        write_string(".ragit/models.json", json.dumps(models))
        cargo_run(["config", "--set", "model", "repeat-after-me"])

        html_context = { "contents": ["<a href=\"wherever\">Hello!</a>"] }
        write_string("html.json", json.dumps(html_context))
        assert "<a href=\"wherever\">" in cargo_run(["pdl", "filter.pdl", "--context=html.json"], stdout=True)
        assert "<a href=\"wherever\">" in cargo_run(["pdl", "no-filter.pdl", "--context=html.json"], stdout=True)

        # I want to make sure `&lt;` in a pdl file is properly handled.
        write_string("unescape.pdl", "<|user|>\n\nrepeat after me: &lt;|schema|&gt;")
        assert "<|schema|>" in cargo_run(["pdl", "unescape.pdl"], stdout=True)

    finally:
        if server_process is not None:
            server_process.kill()
