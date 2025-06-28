# This is a fake LLM server.
# Ideally, I would need to run all the tests with *real* LLMs. However, they are too expensive and slow.
# So, I use a fake LLM that always responds with a dummy response.
# It has a lot of potential: you can add a random delay or rate limits, similar to real LLMs.

from flask import Flask, request
import random
import re
import subprocess
import time
from typing import Callable, Optional, Tuple
from utils import goto_root

app = Flask(__name__)

last_request = 0

# Models
#     rate-limit-<X>: allows at most X requests per minute
#     delay-<X>: sleeps X seconds before it responds
#     delay-<X>-<Y>: sleeps X ~ Y seconds (randomly) before it responds
#     fail-<X>: sends 500 error by X percent
@app.route("/api/chat", methods=["POST"])
def chat():
    j = request.get_json()
    model = j["model"]

    if (r := re.match(r"rate\-limit\-(\d+)", model)) is not None:
        return worker(
            request = j,
            rate_limit = int(r.group(1)),
        )

    elif (r := re.match(r"delay\-(\d+)(?:\-(\d+))?", model)) is not None:
        d_from, d_to = r.groups()
        delay = (lambda: d_from) if d_to is None else (lambda: random.randint(d_from, d_to))
        return worker(
            request = j,
            delay = delay,
        )

    elif (r := re.match(r"fail-(\d+)", model)) is not None:
        return worker(
            request = j,
            fail = lambda: (random.randint(1, 100) <= int(r.group(1)))
        )

    else:
        return worker(j)

# request: {'messages': [{'content': '...', 'role': 'system'}, {'content': '...', 'role': 'user'}], 'model': '...'}
#          {'messages': [{'content': [{'type': 'input_text', 'text': '...'}, {'type': 'input_image', 'image_url': '...'}], ...}]}
def worker(
    request: dict,

    # func(request: dict) -> str
    # If it's None, it returns a string "dummy".
    output_gen: Optional[Callable] = None,

    # func() -> num   # seconds to delay
    # If it's None, it doesn't delay.
    delay: Optional[Callable] = None,

    # If it's set to X, it allows at most X requests per minute.
    rate_limit: Optional[int] = None,

    # func() -> bool  # if true, it returns 500
    fail: Optional[Callable] = None,
) -> Tuple[dict, int]:  # (response, status_code)
    global last_request

    # In real world, a server may fail before it checks
    # rate limit, or after.
    fail_before_rate_limit = random.randint(0, 1) == 1
    if fail: fail = fail()

    if fail_before_rate_limit and fail:
        return {}, 500

    if rate_limit:
        if check_rate_limit(rate_limit):
            return {}, 429

        push_rate_limit_queue()

    if not fail_before_rate_limit and fail:
        return {}, 500

    output_gen = output_gen or (lambda _: "dummy")
    messages = request["messages"]
    input_tokens = 0

    # let's make it as close to real models as possible
    for message in messages:
        content = message["content"]

        if isinstance(content, list):
            content = " ".join([c.get("text", "") for c in content])

        input_tokens += len(content.split(" "))

    output = output_gen(request)
    output_tokens = len(output.split(" "))

    if delay:
        time.sleep(delay())

    return {
        "id": "dummy",
        "object": "dummy",
        "created": int(time.time()),
        "model": request["model"],
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": output,
            },
            "finish_reason": "stop",
        }],
        "usage": {
            "prompt_tokens": input_tokens,
            "completion_tokens": output_tokens,
            "total_tokens": input_tokens + output_tokens,
        },
    }, 200

# dict[minute: int, requests: int]
rate_limit_queue = {}

def check_rate_limit(limit: int) -> bool:
    now = int(time.time()) // 60
    return rate_limit_queue.get(now, 0) >= limit

def push_rate_limit_queue():
    now = int(time.time()) // 60
    rate_limit_queue[now] = rate_limit_queue.get(now, 0) + 1

def host_fake_llm_server():
    goto_root()
    server_process = subprocess.Popen(["python3", "./tests/fake_llm_server.py"])
    return server_process

if __name__ == "__main__":
    # ollama's port number is 11434, so we're using +1
    app.run(host="0.0.0.0", port=11435)
