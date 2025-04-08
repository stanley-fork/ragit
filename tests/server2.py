import os
import requests
from server import create_repo, create_user, health_check
import subprocess
import time
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir

def server2(test_model: str):
    goto_root()
    os.chdir("crates/server")

    if health_check():
        raise Exception("ragit-server is already running. Please run this test in an isolated environment.")

    try:
        # step 0: run a ragit-server
        subprocess.Popen(["cargo", "run", "--release", "--", "truncate-all", "--force"])
        server_process = subprocess.Popen(["cargo", "run", "--release", "--", "run", "--force-default-config"])
        os.chdir("../..")
        mk_and_cd_tmp_dir()

        # before we push this to server, let's wait until `ragit-server` is compiled
        for _ in range(300):
            if health_check():
                break

            print("waiting for ragit-server to start...")
            time.sleep(1)

        else:
            raise Exception("failed to compile `ragit-server`")

        # step 1: init sample 1 (rustc)
        cargo_run(["clone", "http://ragit.baehyunsol.com/sample/rustc", "sample1"])
        os.chdir("sample1")
        cargo_run(["config", "--set", "model", test_model])
        create_user(name="test-user")
        create_repo(user="test-user", repo="sample1")
        cargo_run(["push", "--configs", "--remote=http://127.0.0.1/test-user/sample1"])
        os.chdir("..")

        # step 2: init sample 2 (empty knowledge-base)
        os.mkdir("sample2")
        os.chdir("sample2")
        cargo_run(["init"])
        cargo_run(["config", "--set", "model", test_model])
        create_repo(user="test-user", repo="sample2")
        cargo_run(["push", "--configs", "--remote=http://127.0.0.1/test-user/sample2"])
        os.chdir("..")

        # step 3: let's ask questions!
        chat_id1 = requests.post("http://127.0.0.1:41127/test-user/sample1/chat-list").text
        chat_id2 = requests.post("http://127.0.0.1:41127/test-user/sample2/chat-list").text
        responses1 = []
        responses2 = []

        chat_list = requests.get("http://127.0.0.1:41127/test-user/sample1/chat-list").json()
        assert len(chat_list) == 1
        assert str(chat_list[0]["id"]) == chat_id1

        # TODO: what's the difference between multipart/form and body/form? I'm noob to this...
        responses1.append(requests.post(f"http://127.0.0.1:41127/test-user/sample1/chat/{chat_id1}", files={"query": "How does the rust compiler implement type system?"}).json())
        responses2.append(requests.post(f"http://127.0.0.1:41127/test-user/sample2/chat/{chat_id2}", files={"query": "How does the rust compiler implement type system?"}).json())

        responses1.append(requests.post(f"http://127.0.0.1:41127/test-user/sample1/chat/{chat_id1}", data={"query": "What do you mean by MIR?"}).json())
        responses2.append(requests.post(f"http://127.0.0.1:41127/test-user/sample2/chat/{chat_id2}", data={"query": "What do you mean by MIR?"}).json())

        responses1.append(requests.post(f"http://127.0.0.1:41127/test-user/sample1/chat/{chat_id1}", files={"query": "Thanks!"}).json())
        responses2.append(requests.post(f"http://127.0.0.1:41127/test-user/sample2/chat/{chat_id2}", files={"query": "Thanks!"}).json())

        history1 = requests.get(f"http://127.0.0.1:41127/test-user/sample1/chat/{chat_id1}").json()["history"]
        history2 = requests.get(f"http://127.0.0.1:41127/test-user/sample2/chat/{chat_id2}").json()["history"]

        chat_list = requests.get("http://127.0.0.1:41127/test-user/sample1/chat-list").json()
        assert len(chat_list) == 1
        assert str(chat_list[0]["id"]) == chat_id1

        for response in responses2:
            assert len(response["retrieved_chunks"]) == 0

        assert [h["response"] for h in history1] == responses1
        assert [h["response"] for h in history2] == responses2

        # ii-build is idempotent
        for _ in range(3):
            assert requests.post("http://127.0.0.1:41127/test-user/sample1/ii-build").status_code == 200
            assert requests.post("http://127.0.0.1:41127/test-user/sample2/ii-build").status_code == 200

    finally:
        server_process.kill()
