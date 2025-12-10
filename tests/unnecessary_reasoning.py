import os
import shutil
import time
from utils import (
    cargo_run,
    goto_root,
    ls_recursive,
    mk_and_cd_tmp_dir,
)

def test(models: list[str]):
    goto_root()
    mk_and_cd_tmp_dir("__tmp123__")

    cargo_run(["clone", "https://ragit.baehyunsol.com/sample/ragit"])
    os.chdir("ragit")
    result = ""

    for model in models:
        result += "---------------------------\n"
        result += f"----- model: {model} -----\n"
        cargo_run(["config", "--set", "model", model])
        cargo_run(["config", "--set", "summary_after_build", "false"])

        started_at = time.time()
        s = cargo_run(["query", "`rag query` command is sometimes terribly slow. Why?"], output_schema=["returncode", "stdout", "stderr"], check=False)
        s = s["stdout"] if s["returncode"] == 0 else s["stderr"]

        elapsed = time.time() - started_at
        result += f"-- query ({elapsed:.2f}s) --\n"
        result += s + "\n"

        started_at = time.time()
        s = cargo_run(["summary", "--force"], output_schema=["returncode", "stdout", "stderr"], check=False)
        s = s["stdout"] if s["returncode"] == 0 else s["stderr"]
        elapsed = time.time() - started_at
        result += f"-- summary ({elapsed:.2f}s) --\n"
        result += s + "\n"

        os.chdir("../../docs/")

        if os.path.exists(".ragit"):
            shutil.rmtree(".ragit")

        cargo_run(["init"])
        cargo_run(["config", "--set", "model", model])
        cargo_run(["config", "--set", "summary_after_build", "false"])
        files = ls_recursive("txt") + ls_recursive("md")
        cargo_run(["add", *files])

        started_at = time.time()
        cargo_run(["build"], stdout=True)
        elapsed = time.time() - started_at
        result += f"-- build ({elapsed:.2f}s) --\n"

        os.chdir("../__tmp123__/ragit")

        with open("../../tests/unrs.txt", "w") as f:
            f.write(result)

# openai: gpt-4o, gpt-4o-mini, gpt-5, gpt-5-mini, gpt-oss-20b-groq, gpt-oss-120b-groq, llama3.3-70b-groq
# google: gemini-2.0-flash, gemini-2.5-pro, gemini-2.5-flash, gemini-3.0-pro
# anthropic: claude-4-sonnet, claude-4.5-haiku, claude-4.5-sonnet, claude-4.5-opus

models = [
    "gpt-4o", "gpt-4o-mini", "gpt-5", "gpt-5-mini",
    "gpt-oss-20b-groq", "gpt-oss-120b-groq", "llama3.3-70b-groq",
    # NOTE: gemini-3.0-pro is still in preview
    "gemini-2.0-flash", "gemini-2.5-pro", "gemini-2.5-flash", # "gemini-3.0-pro",
    "claude-4-sonnet", "claude-4.5-haiku", "claude-4.5-sonnet", "claude-4.5-opus",
]
test(models)

# t1: `reasoning_effort=low` for openai, `thinking={type=disabled}` for anthropic
# -> unrs-1.txt
