import json
import os
import requests
import subprocess
import time
from typing import Optional
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    read_string,
    write_string,
)

# It does not test api endpoints that are tested by `clone.py` or `server2.py`.
def server():
    goto_root()

    if health_check():
        raise Exception("ragit-server is already running. Please run this test in an isolated environment.")

    os.chdir("crates/server")

    try:
        # step 0: run a ragit-server
        subprocess.Popen(["cargo", "run", "--release", "--", "truncate-all", "--force"])
        server_process = subprocess.Popen(["cargo", "run", "--release", "--", "run", "--force-default-config"])
        os.chdir("../..")
        mk_and_cd_tmp_dir()

        # step 1: we'll do experiments with these repos
        cargo_run(["clone", "http://ragit.baehyunsol.com/sample/rustc", "sample-rustc"])

        os.mkdir("sample-empty")
        os.chdir("sample-empty")
        cargo_run(["init"])
        os.chdir("..")

        os.mkdir("sample-small")
        os.chdir("sample-small")
        cargo_run(["init"])
        cargo_run(["config", "--set", "model", "dummy"])
        write_string("sample.md", "Hello, World!")
        cargo_run(["add", "sample.md"])
        cargo_run(["build"])
        os.chdir("..")

        # step 2: test loop
        for repo in ["sample-empty", "sample-small", "sample-rustc"]:
            os.chdir(repo)
            sample_chunk_uid = json.loads(cargo_run(["ls-chunks", "--json"], stdout=True))[0]["uid"] if repo == "sample-rustc" else None
            sample_file_uid = [file for file in json.loads(cargo_run(["ls-files", "--json"], stdout=True)) if file["chunks"] > 1][0]["uid"] if repo == "sample-rustc" else None

            # before we push this to server, let's wait until `ragit-server` is compiled
            for _ in range(300):
                if health_check():
                    break

                print("waiting for ragit-server to start...")
                time.sleep(1)

            else:
                raise Exception("failed to compile `ragit-server`")

            cargo_run(["meta", "--set", "whatever-key", "whatever-value"])
            cargo_run(["push", "--configs", "--prompts", f"--remote=http://127.0.0.1/test-user/{repo}/"])
            index_json = request_json("index", repo)
            assert_eq_json("index.json", index_json)

            for config in ["build", "api", "query"]:
                config_json = request_json(f"config/{config}", repo)
                assert_eq_json(os.path.join("configs", f"{config}.json"), config_json)

            for prompt in os.listdir(os.path.join(".ragit", "prompts")):
                prompt_name = os.path.basename(prompt)
                prompt_pdl = request_text(f"prompt/{prompt_name[:-4]}", repo)
                assert_eq_text(os.path.join("prompts", prompt_name), prompt_pdl)

            chunk_count = request_json("chunk-count", repo)
            assert chunk_count == len(json.loads(cargo_run(["ls-chunks", "--json"], stdout=True)))

            all_chunk_uids = request_json("chunk-list", repo)
            all_chunk_uids_local = json.loads(cargo_run(["ls-chunks", "--json", "--uid-only"], stdout=True))
            assert set(all_chunk_uids) == set(all_chunk_uids_local)

            all_image_uids = json.loads(cargo_run(["ls-images", "--json", "--uid-only"], stdout=True))

            for chunk_uid in all_chunk_uids:
                chunk = request_json(f"chunk/{chunk_uid}", repo)
                chunk_local = json.loads(cargo_run(["ls-chunks", "--json", chunk_uid], stdout=True))[0]

                # due to prettifier, the json values are a bit different
                assert chunk["data"] == chunk_local["data"]

            for image_uid in all_image_uids:
                image = request_bytes(f"image/{image_uid}", repo)
                assert_eq_bytes(os.path.join("images", image_uid[:2], f"{image_uid[2:]}.png"), image)
                image_desc = request_json(f"image-desc/{image_uid}", repo)
                image_desc_local = json.loads(cargo_run(["ls-images", "--json", image_uid], stdout=True))[0]

                assert image_desc["explanation"] == image_desc_local["explanation"]
                assert image_desc["extracted_text"] == image_desc_local["extracted_text"]

            for prefix in range(256):
                prefix = f"{prefix:02x}"

                if any([uid.startswith(prefix) for uid in all_chunk_uids]):
                    uids_from_api = request_json(f"chunk-list/{prefix}", repo)
                    uids_local = json.loads(cargo_run(["ls-chunks", "--json", "--uid-only", prefix], stdout=True))
                    assert set(uids_from_api) == set(uids_local)

                else:
                    assert request_json(f"chunk-list/{prefix}", repo) == []

                if any([uid.startswith(prefix) for uid in all_image_uids]):
                    uids_from_api = request_json(f"image-list/{prefix}", repo)
                    uids_local = json.loads(cargo_run(["ls-images", "--json", "--uid-only", prefix], stdout=True))
                    assert set(uids_from_api) == set(uids_local)

                else:
                    assert request_json(f"image-list/{prefix}", repo) == []

            file_api = request_json("file-list", repo)
            file_local = json.loads(cargo_run(["ls-files", "--name-only", "--json"], stdout=True))
            assert set(file_api) == set(file_local)

            meta_api = request_json("meta", repo)
            assert_eq_json("meta.json", meta_api)

            version_api = request_text("version", repo).strip()
            version_local = json.loads(read_string(os.path.join(".ragit", "index.json")))["ragit_version"]
            assert version_api == version_local

            version_api = request_text("http://127.0.0.1:41127/version", repo, raw_url=True).strip()
            version_local = cargo_run(["version"], stdout=True).strip()
            assert version_api in version_local

            user_list = request_json("http://127.0.0.1:41127/user-list", repo, raw_url=True)
            assert "test-user" in user_list

            repo_list = request_json("http://127.0.0.1:41127/repo-list/test-user", repo, raw_url=True)
            assert repo in repo_list

            if sample_chunk_uid is not None:
                cat_file_chunk_api = request_text(f"cat-file/{sample_chunk_uid}", repo).strip()
                cat_file_chunk_local = cargo_run(["cat-file", sample_chunk_uid], stdout=True).strip()
                assert cat_file_chunk_api == cat_file_chunk_local

            if sample_file_uid is not None:
                cat_file_file_api = request_text(f"cat-file/{sample_file_uid}", repo).strip()
                cat_file_file_local = cargo_run(["cat-file", sample_file_uid], stdout=True).strip()
                assert cat_file_file_api == cat_file_file_local

            os.chdir("..")

    finally:
        server_process.kill()

def create_user(
    name: str,
    email: Optional[str] = None,
    password: str = "12345678",
    readme: Optional[str] = None,
    public: bool = True,
) -> int:  # returns user_id
    body = {
        "name": name,
        "email": email,
        "password": password,
        "readme": readme,
        "public": public,
    }
    response = requests.post("http://127.0.0.1:41127/user-list", json=body)
    assert response.status_code == 200
    return response.json()

def create_repo(
    user: str,
    repo: str,
    description: Optional[str] = None,
    website: Optional[str] = None,
    readme: Optional[str] = None,
    public_read: bool = True,
    public_write: bool = True,
    public_clone: bool = True,
    public_push: bool = True,
) -> int:  # returns repo_id
    body = {
        "name": repo,
        "description": description,
        "website": website,
        "readme": readme,
        "public_read": public_read,
        "public_write": public_write,
        "public_clone": public_clone,
        "public_push": public_push,
    }
    response = requests.post(f"http://127.0.0.1:41127/repo-list/{user}", json=body)
    assert response.status_code == 200
    return response.json()

def request_json(url: str, repo: str, raw_url: bool = False):
    response = requests.get(os.path.join(f"http://127.0.0.1:41127/test-user/{repo}", url) if not raw_url else url)
    assert response.status_code == 200
    return json.loads(response.text)

def assert_eq_json(path: str, value):
    file = json.loads(read_string(os.path.join(".ragit", path)))

    if file != value:
        raise ValueError(f"{file.__repr__()} != {value.__repr__()}")

def request_text(url: str, repo: str, raw_url: bool = False) -> str:
    response = requests.get(os.path.join(f"http://127.0.0.1:41127/test-user/{repo}", url) if not raw_url else url)
    assert response.status_code == 200
    return response.text

def assert_eq_text(path: str, value: str):
    file = read_string(os.path.join(".ragit", path))

    if file != value:
        raise ValueError(f"{file.__repr__()} != {value.__repr__()}")

def request_bytes(url: str, repo: str, raw_url: bool = False):
    response = requests.get(os.path.join(f"http://127.0.0.1:41127/test-user/{repo}", url) if not raw_url else url)
    assert response.status_code == 200
    return response.content

def assert_eq_bytes(path: str, value):
    with open(os.path.join(".ragit", path), "rb") as f:
        assert f.read() == value

def health_check(port: int = 41127):
    try:
        response = requests.get(f"http://127.0.0.1:{port}/health", timeout=1)
        return response.status_code == 200

    except:
        return False
