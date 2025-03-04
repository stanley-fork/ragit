import json
import os
import requests
import subprocess
import time
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, read_string

# It does not test api endpoints that are tested by `clone.py` or `server2.py`.
def server():
    goto_root()
    os.chdir("crates/server")

    try:
        # step 0: run a ragit-server
        server_process = subprocess.Popen(["cargo", "run", "--release"])
        os.chdir("../..")
        mk_and_cd_tmp_dir()

        # step 1: we'll do experiments with this repo
        cargo_run(["clone", "http://ragit.baehyunsol.com/sample/rustc", "sample"])
        os.chdir("sample")

        sample_chunk_uid = json.loads(cargo_run(["ls-chunks", "--json"], stdout=True))[0]["uid"]
        sample_file_uid = [file for file in json.loads(cargo_run(["ls-files", "--json"], stdout=True)) if file["chunks"] > 1][0]["uid"]

        # before we push this to server, let's wait until `ragit-server` is compiled
        for _ in range(300):
            path1 = "../../crates/server/target/release/ragit-server"
            path2 = "../../crates/server/target/release/ragit-server.exe"

            if not os.path.exists(path1) and not os.path.exists(path2):
                time.sleep(1)

            else:
                break

        else:
            raise Exception("failed to compile `ragit-server`")

        cargo_run(["meta", "--set", "whatever-key", "whatever-value"])
        cargo_run(["push", "--configs", "--prompts", "--remote=http://127.0.0.1/test-user/repo1/"])
        index_json = request_json("index")
        assert_eq_json("index.json", index_json)

        for config in ["build", "api", "query"]:
            config_json = request_json(f"config/{config}")
            assert_eq_json(os.path.join("configs", f"{config}.json"), config_json)

        for prompt in os.listdir(os.path.join(".ragit", "prompts")):
            prompt_name = os.path.basename(prompt)
            prompt_pdl = request_text(f"prompt/{prompt_name[:-4]}")
            assert_eq_text(os.path.join("prompts", prompt_name), prompt_pdl)

        chunk_count = request_json("chunk-count")
        assert chunk_count == len(json.loads(cargo_run(["ls-chunks", "--json"], stdout=True)))

        all_chunk_uids = request_json("chunk-list")
        all_chunk_uids_local = json.loads(cargo_run(["ls-chunks", "--json", "--uid-only"], stdout=True))
        assert set(all_chunk_uids) == set(all_chunk_uids_local)

        all_image_uids = json.loads(cargo_run(["ls-images", "--json", "--uid-only"], stdout=True))

        for chunk_uid in all_chunk_uids:
            chunk = request_json(f"chunk/{chunk_uid}")
            chunk_local = json.loads(cargo_run(["ls-chunks", "--json", chunk_uid], stdout=True))[0]

            # due to prettifier, the json values are a bit different
            assert chunk["data"] == chunk_local["data"]

        for image_uid in all_image_uids:
            image = request_bytes(f"image/{image_uid}")
            assert_eq_bytes(os.path.join("images", image_uid[:2], f"{image_uid[2:]}.png"), image)
            image_desc = request_json(f"image-desc/{image_uid}")
            image_desc_local = json.loads(cargo_run(["ls-images", "--json", image_uid], stdout=True))[0]

            assert image_desc["explanation"] == image_desc_local["explanation"]
            assert image_desc["extracted_text"] == image_desc_local["extracted_text"]

        for prefix in range(256):
            prefix = f"{prefix:02x}"

            if any([uid.startswith(prefix) for uid in all_chunk_uids]):
                uids_from_api = request_json(f"chunk-list/{prefix}")
                uids_local = json.loads(cargo_run(["ls-chunks", "--json", "--uid-only", prefix], stdout=True))
                assert set(uids_from_api) == set(uids_local)

            if any([uid.startswith(prefix) for uid in all_image_uids]):
                uids_from_api = request_json(f"image-list/{prefix}")
                uids_local = json.loads(cargo_run(["ls-images", "--json", "--uid-only", prefix], stdout=True))
                assert set(uids_from_api) == set(uids_local)

        meta_api = request_json("meta")
        assert_eq_json("meta.json", meta_api)

        version_api = request_text("version").strip()
        version_local = json.loads(read_string(os.path.join(".ragit", "index.json")))["ragit_version"]
        assert version_api == version_local

        version_api = request_text("http://127.0.0.1:41127/version", raw_url=True).strip()
        version_local = cargo_run(["version"], stdout=True).strip()
        assert version_api in version_local

        user_list = request_json("http://127.0.0.1:41127/user-list", raw_url=True)
        assert "test-user" in user_list

        repo_list = request_json("http://127.0.0.1:41127/repo-list/test-user", raw_url=True)
        assert "repo1" in repo_list

        cat_file_chunk_api = request_text(f"cat-file/{sample_chunk_uid}").strip()
        cat_file_chunk_local = cargo_run(["cat-file", sample_chunk_uid], stdout=True).strip()
        assert cat_file_chunk_api == cat_file_chunk_local

        cat_file_file_api = request_text(f"cat-file/{sample_file_uid}").strip()
        cat_file_file_local = cargo_run(["cat-file", sample_file_uid], stdout=True).strip()
        assert cat_file_file_api == cat_file_file_local

    finally:
        server_process.kill()

def request_json(url: str, raw_url: bool = False):
    response = requests.get(os.path.join("http://127.0.0.1:41127/test-user/repo1", url) if not raw_url else url)
    assert response.status_code == 200
    return json.loads(response.text)

def assert_eq_json(path: str, value):
    file = json.loads(read_string(os.path.join(".ragit", path)))

    if file != value:
        raise ValueError(f"{file.__repr__()} != {value.__repr__()}")

def request_text(url: str, raw_url: bool = False) -> str:
    response = requests.get(os.path.join("http://127.0.0.1:41127/test-user/repo1", url) if not raw_url else url)
    assert response.status_code == 200
    return response.text

def assert_eq_text(path: str, value: str):
    file = read_string(os.path.join(".ragit", path))

    if file != value:
        raise ValueError(f"{file.__repr__()} != {value.__repr__()}")

def request_bytes(url: str, raw_url: bool = False):
    response = requests.get(os.path.join("http://127.0.0.1:41127/test-user/repo1", url) if not raw_url else url)
    assert response.status_code == 200
    return response.content

def assert_eq_bytes(path: str, value):
    with open(os.path.join(".ragit", path), "rb") as f:
        assert f.read() == value
