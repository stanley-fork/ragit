import json
import os
import requests
import subprocess
import time
from typing import Optional, Tuple
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    read_string,
    write_string,
)

def server():
    goto_root()
    server_process = None

    try:
        server_process = spawn_ragit_server()
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
        create_user(id="test-user", password="12345678")
        api_key = get_api_key(id="test-user", password="12345678")

        # step 2: test loop
        for index, repo in enumerate(["sample-empty", "sample-small", "sample-rustc"]):
            os.chdir(repo)
            sample_chunk_uids = [chunk["uid"] for chunk in json.loads(cargo_run(["ls-chunks", "--json"], stdout=True))[:5]]
            sample_file_uids = [file["uid"] for file in json.loads(cargo_run(["ls-files", "--json"], stdout=True)) if file["chunks"] > 1][:5]

            cargo_run(["meta", "--set", "whatever-key", "whatever-value"])
            create_repo(user="test-user", repo=repo, api_key=api_key, public_read=True, public_write=True)
            cargo_run(["uid"])  # `index.json` has `uid` field. in order to `assert_eq_json`, we have to init uid of the local index
            cargo_run(["push", "--configs", "--prompts", f"--remote=http://127.0.0.1/test-user/{repo}/"])
            index_json = get_json(url="index", repo=repo)
            assert_eq_json("index.json", index_json)

            for config in ["build", "api", "query"]:
                config_json = get_json(url=f"config/{config}", repo=repo)
                assert_eq_json(os.path.join("configs", f"{config}.json"), config_json)

            for prompt in os.listdir(os.path.join(".ragit", "prompts")):
                prompt_name = os.path.basename(prompt)
                prompt_pdl = request_text(url=f"prompt/{prompt_name[:-4]}", repo=repo)
                assert_eq_text(os.path.join("prompts", prompt_name), prompt_pdl)

            chunk_count = get_json(url="chunk-count", repo=repo)
            assert chunk_count == len(json.loads(cargo_run(["ls-chunks", "--json"], stdout=True)))

            all_chunk_uids = get_json(url="chunk-list", repo=repo)
            all_chunk_uids_local = json.loads(cargo_run(["ls-chunks", "--json", "--uid-only"], stdout=True))
            assert set(all_chunk_uids) == set(all_chunk_uids_local)

            all_image_uids = json.loads(cargo_run(["ls-images", "--json", "--uid-only"], stdout=True))

            for chunk_uid in all_chunk_uids:
                chunk = get_json(url = f"chunk/{chunk_uid}", repo = repo)
                chunk_local = json.loads(cargo_run(["ls-chunks", "--json", chunk_uid], stdout=True))[0]

                # due to prettifier, the json values are a bit different
                assert compare_chunk_data(chunk["data"], chunk_local["data"])

            for image_uid in all_image_uids:
                image = request_bytes(url=f"image/{image_uid}", repo=repo)
                assert_eq_bytes(os.path.join("images", image_uid[:2], f"{image_uid[2:]}.png"), image)
                image_desc = get_json(url=f"image-desc/{image_uid}", repo=repo)
                image_desc_local = json.loads(cargo_run(["ls-images", "--json", image_uid], stdout=True))[0]

                assert image_desc["explanation"] == image_desc_local["explanation"]
                assert image_desc["extracted_text"] == image_desc_local["extracted_text"]

            for prefix in range(256):
                prefix = f"{prefix:02x}"

                if any([uid.startswith(prefix) for uid in all_chunk_uids]):
                    uids_from_api = get_json(url=f"chunk-list/{prefix}", repo=repo)
                    uids_local = json.loads(cargo_run(["ls-chunks", "--json", "--uid-only", prefix], stdout=True))
                    assert set(uids_from_api) == set(uids_local)

                else:
                    assert get_json(url=f"chunk-list/{prefix}", repo=repo) == []

                if any([uid.startswith(prefix) for uid in all_image_uids]):
                    uids_from_api = get_json(url=f"image-list/{prefix}", repo=repo)
                    uids_local = json.loads(cargo_run(["ls-images", "--json", "--uid-only", prefix], stdout=True))
                    assert set(uids_from_api) == set(uids_local)

                else:
                    assert get_json(url=f"image-list/{prefix}", repo=repo) == []

            meta_api = get_json(url="meta", repo=repo)
            assert_eq_json("meta.json", meta_api)

            meta_by_key_api = get_json(url="meta/whatever-key", repo=repo)
            assert meta_by_key_api == "whatever-value"

            meta_by_key_api = get_json(url="meta/invalid-key", repo=repo)
            assert meta_by_key_api is None

            version_api = request_text(url="version", repo=repo).strip()
            version_local = json.loads(read_string(os.path.join(".ragit", "index.json")))["ragit_version"]
            assert version_api == version_local

            version_api = request_text(url="http://127.0.0.1:41127/version", repo=repo, raw_url=True).strip()
            version_local = cargo_run(["version"], stdout=True).strip()
            assert version_api in version_local

            user_list = get_json(url="http://127.0.0.1:41127/user-list", repo=repo, raw_url=True)
            assert len(user_list) == 1 and user_list[0]["id"] == "test-user"

            repo_list = get_json(url="http://127.0.0.1:41127/repo-list/test-user", repo=repo, raw_url=True)
            assert len(repo_list) == index + 1 and any([r["name"] == repo for r in repo_list]) and all([r["owner"] == "test-user" for r in repo_list])

            for sample_chunk_uid in sample_chunk_uids:
                cat_file_chunk_api = request_text(url=f"cat-file/{sample_chunk_uid}", repo=repo).strip()
                cat_file_chunk_local = cargo_run(["cat-file", sample_chunk_uid], stdout=True).strip()
                get_content_api = get_json(url=f"content/{sample_chunk_uid}", repo=repo)
                get_content_local = json.loads(cargo_run(["cat-file", "--json", sample_chunk_uid], stdout=True))
                assert cat_file_chunk_api == cat_file_chunk_local
                assert get_content_api == get_content_local

            for sample_file_uid in sample_file_uids:
                cat_file_file_api = request_text(url=f"cat-file/{sample_file_uid}", repo=repo).strip()
                cat_file_file_local = cargo_run(["cat-file", sample_file_uid], stdout=True).strip()
                get_content_api = get_json(url=f"content/{sample_file_uid}", repo=repo)
                get_content_local = json.loads(cargo_run(["cat-file", "--json", sample_file_uid], stdout=True))
                assert cat_file_file_api == cat_file_file_local
                assert get_content_api == get_content_local

            os.chdir("..")

    finally:
        if server_process is not None:
            server_process.kill()

# 0. truncate all the data in the server
# 1. spawns a ragit-server process
# 2. waits until the server becomes healthy
# 3. go to root and return the server process
def spawn_ragit_server():
    goto_root()

    if health_check():
        raise Exception("ragit-server is already running. Please run this test in an isolated environment.")

    os.chdir("crates/server")
    subprocess.Popen(["cargo", "run", "--release", "--", "truncate-all", "--force"]).wait()
    server_process = subprocess.Popen(["cargo", "run", "--release", "--features=log_sql", "--", "run", "--force-default-config"])

    for _ in range(300):
        if health_check():
            break

        print("waiting for ragit-server to start...")
        time.sleep(1)

    else:
        raise Exception("failed to run `ragit-server`")

    os.chdir("../..")
    return server_process

def create_user(
    id: str,
    password: str = "12345678",
    email: str = "sample@email.com",
    readme: Optional[str] = None,
    public: bool = True,
    api_key: Optional[str] = None,
    expected_status_code: Optional[int] = 200,
):
    body = {
        "id": id,
        "password": password,
        "email": email,
        "readme": readme,
        "public": public,
    }
    post_json(
        url="http://127.0.0.1:41127/user-list",
        body=body,
        raw_url=True,
        api_key=api_key,
        expected_status_code=expected_status_code,
    )

def create_repo(
    user: str,
    repo: str,
    api_key: Optional[str] = None,
    description: Optional[str] = None,
    website: Optional[str] = None,
    readme: Optional[str] = None,
    public_read: bool = True,
    public_write: bool = True,
    public_clone: bool = True,
    public_push: bool = True,
    public_chat: bool = True,
    expected_status_code: Optional[int] = 200,
) -> Optional[int]:  # returns repo_id, if successful
    body = {
        "name": repo,
        "description": description,
        "website": website,
        "readme": readme,
        "public_read": public_read,
        "public_write": public_write,
        "public_clone": public_clone,
        "public_push": public_push,
        "public_chat": public_chat,
    }
    headers = {} if api_key is None else { "x-api-key": api_key }
    response = requests.post(f"http://127.0.0.1:41127/repo-list/{user}", json=body, headers=headers)

    if expected_status_code is not None:
        assert response.status_code == expected_status_code

    if expected_status_code == 200:
        return response.json()

def get_repo_stat(
    user: str,
    repo: str,
    key: str = "all",
    expected_status_code: Optional[int] = 200,
) -> Tuple[int, int]:  # returns (push, clone)
    response = requests.get(f"http://127.0.0.1:41127/{user}/{repo}/traffic")

    if expected_status_code is not None:
        assert response.status_code == expected_status_code

    if expected_status_code == 200:
        response = response.json()
        return (response[key]["push"], response[key]["clone"])

    else:
        return (0, 0)

def get_api_key(
    id: str,
    password: str = "12345678",
    name: str = "login_token",
) -> str:
    body = {
        "name": name,
        "expire_after": 14,
        "password": password,
    }
    response = requests.post(f"http://127.0.0.1:41127/user-list/{id}/api-key-list", json=body)
    assert response.status_code == 200
    return response.text

def post_json(
    url: str,
    body,
    user: Optional[str] = "test-user",
    repo: Optional[str] = None,
    raw_url: bool = False,
    api_key: Optional[str] = None,
    expected_status_code: Optional[int] = 200,
):
    headers = {} if api_key is None else { "x-api-key": api_key }
    response = requests.post(
        os.path.join(f"http://127.0.0.1:41127/{user}/{repo}", url) if not raw_url else url,
        json=body,
        headers=headers,
    )

    if expected_status_code is not None:
        assert response.status_code == expected_status_code

def get_json(
    url: str,
    user: Optional[str] = "test-user",
    repo: Optional[str] = None,
    raw_url: bool = False,
    api_key: Optional[str] = None,
    expected_status_code: Optional[int] = 200,
):
    headers = {} if api_key is None else { "x-api-key": api_key }
    response = requests.get(
        os.path.join(f"http://127.0.0.1:41127/{user}/{repo}", url) if not raw_url else url,
        headers=headers,
    )

    if expected_status_code is not None:
        assert response.status_code == expected_status_code

    if expected_status_code == 200:
        return json.loads(response.text)

def assert_eq_json(path: str, value):
    file = json.loads(read_string(os.path.join(".ragit", path)))

    if file != value:
        raise ValueError(f"{file.__repr__()} != {value.__repr__()}")

def request_text(
    url: str,
    user: Optional[str] = "test-user",
    repo: Optional[str] = None,
    raw_url: bool = False,
    expected_status_code: Optional[int] = 200,
) -> str:
    response = requests.get(os.path.join(f"http://127.0.0.1:41127/{user}/{repo}", url) if not raw_url else url)

    if expected_status_code is not None:
        assert response.status_code == expected_status_code

    if expected_status_code == 200:
        return response.text

    else:
        return ""

def assert_eq_text(path: str, value: str):
    file = read_string(os.path.join(".ragit", path))

    if file != value:
        raise ValueError(f"{file.__repr__()} != {value.__repr__()}")

def request_bytes(
    url: str,
    user: Optional[str] = "test-user",
    repo: Optional[str] = None,
    raw_url: bool = False,
    expected_status_code: Optional[int] = 200,
) -> bytes:
    response = requests.get(os.path.join(f"http://127.0.0.1:41127/{user}/{repo}", url) if not raw_url else url)

    if expected_status_code is not None:
        assert response.status_code == expected_status_code

    if expected_status_code == 200:
        return response.content

    else:
        return b""

def assert_eq_bytes(path: str, value: bytes):
    with open(os.path.join(".ragit", path), "rb") as f:
        assert f.read() == value

def health_check(port: int = 41127):
    try:
        response = requests.get(f"http://127.0.0.1:{port}/health", timeout=1)
        return response.status_code == 200

    except:
        return False

# There are too many abstractions and fragmentations :(
def compare_chunk_data(data_from_api, data_from_local) -> bool:
    return data_from_local == "".join([(data["content"] if data["type"] == "Text" else ("img_" + data["uid"])) for data in data_from_api])
