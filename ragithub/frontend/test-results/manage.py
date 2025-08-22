from hashlib import sha3_256
import json
import os
import re
import requests
import shutil
import subprocess
import sys
from typing import Optional

def deepcopy(v):
    return eval(str(v))

def get_commit_info(hash: str) -> dict:
    with open(os.path.join("commits", hash[:2], hash[2:] + ".json"), "r") as f:
        return json.loads(f.read())

# r -> content of `ragit/tests/result.json`
def get_file_name(r: dict) -> str:
    hash = r["meta"]["commit"]
    os = r["meta"]["platform"]["platform"].lower()
    os = "windows" if "windows" in os else "mac" if "mac" in os else "linux" if "linux" in os else None
    return f"result-{hash[:9]}-{os}.json"

# dump format of `ragit/tests/result.json` changed over time.
#
# Some results are pointing to invalid commit hashes (my mistake
# when running the test harness). In those cases, it returns None.
#
# Some very old versions might be missing necessary information.
# It also returns None for those cases.
def normalize_test_result(r: dict) -> Optional[dict]:
    r = deepcopy(r)

    try:
        commit_info = get_commit_info(r["meta"]["commit"][:9])

    except Exception as e:
        print(f"Cannot get commit info of {r['meta']['commit'][:9]}: {e}")
        return None

    if "ragit-version" in r["meta"]:
        r["meta"]["ragit_version"] = r["meta"].pop("ragit-version")

    if "rand_seed" not in r["meta"]:
        r["meta"]["rand_seed"] = 0

    r["meta"]["commit_title"] = commit_info["title"]
    r["meta"]["commit_message"] = commit_info["message"]

    if "Z" not in r["meta"]["started_at"]:
        r["meta"]["started_at"] += "Z"

    if "Z" not in r["meta"]["ended_at"]:
        r["meta"]["ended_at"] += "Z"

    return r

def get_test_descriptions() -> dict[str, str]:  # dict[test_name, description]
    # I don't want to import this script. I don't want to execute any line of ragit.
    with open("../../ragit/tests/tests.py", "r") as f:
        d = f.read()

    help_message = re.search(r'help_message\s=\s"""(.+)"""', d, flags=re.DOTALL).group(1)
    help_message = help_message.strip()
    assert help_message.startswith("Commands\n")
    help_message = help_message[len("Commands\n"):]

    descriptions = help_message.split("\n\n")
    descriptions = [d for d in descriptions if d.strip() != ""]
    return { [dd for dd in d.split(" ") if dd != ""][0]: d for d in descriptions }

help_message = """
Whatever you're doing, please make sure that you're at `ragithub/test-results/` and `../../ragit/` exists.

# How to add new test result

1. On whatever machine
  - Run test. Make sure to checkout to the most recent commit after you run the test.
  - Copy `result.json` to `ragithub/test-results/result.json`.
  - Run `python3 manage.py fetch_git_info`.
  - Run `python3 manage.py import`.
  - Git add, commit and push.
2. On the server
  - Run `git pull` (in `ragithub/`, not in `ragit/`).
  - Run `python3 manage.py fetch_git_info`.
  - Run `python3 manage.py create_index`.

# How to delete an existing test result

Remove the file in the file system and run `python3 manage.py create_index`.

The remaining are the same as adding a result.

# If there're some updates with manage.py

Run `python3 manage.py import --force` to change the schema of the result files.
"""

if __name__ == "__main__":
    command = None if len(sys.argv) < 2 else sys.argv[1]

    if command == "import_legacy":
        for i in range(100, 0, -1):
            try:
                r = requests.get(url=f"http://ragit.baehyunsol.com/download/json/{i}")
                r = r.json()

            except:
                assert "404" in r.text
                continue

            file_name = get_file_name(r)
            r = normalize_test_result(r)

            if r is not None:
                with open(file_name, "w") as f:
                    f.write(json.dumps(r, ensure_ascii=False, indent=4))
                    print(file_name)

    # When you run a new test, `cp ../ragit/tests/result.json .` and run this command.
    elif command == "import":
        force = "--force" in sys.argv

        for file in os.listdir():
            if not file.endswith(".json") or file == "_index.json":
                continue

            # assumption: if file name is normalized, its content must be normalized
            if not force and re.match(r"^result\-([0-9a-f]{9}\-[a-z]+)\.json$", file) is not None:
                continue

            with open(file, "r") as f:
                r = json.loads(f.read())

            file_name = get_file_name(r)
            r = normalize_test_result(r)

            if r is not None:
                with open(file_name, "w") as f:
                    f.write(json.dumps(r, ensure_ascii=False, indent=4))

            if file != file_name:
                os.remove(file)

    # It assumes that `../../ragit/tests/tests.py` exists.
    # It assumes that all files have valid name and format.
    # You have to run `fetch_git_info` and `import` before running this command.
    elif command == "create_index":
        index = []
        case_history = {}
        timestamp_by_title_map = {}
        name_by_name_hash_map = {}
        description_by_name_map = get_test_descriptions()

        for file in os.listdir():
            if not file.endswith(".json") or file == "_index.json":
                continue

            with open(file, "r") as f:
                j = json.load(f)

            if j["result"]["remaining"] > 0:
                print(f"{file} is an incomplete result!")
                continue

            title = re.match(r"^result\-([0-9a-f]{9}\-[a-z]+)\.json$", file).group(1)
            timestamp_by_title_map[title] = j["meta"]["ended_at"]

            index.append({
                "git_title": get_commit_info(title[:9])["title"],
                "title": title,
                "ended_at": j["meta"]["ended_at"],
                "ragit_version": j["meta"]["ragit_version"],
                "pass": j["result"]["pass"],
                "fail": j["result"]["fail"],
            })

            for (name, result) in j["tests"].items():
                name_hash = sha3_256(name.encode("utf-8")).hexdigest()[:9]
                name_by_name_hash_map[name_hash] = name
                case_history[name_hash] = case_history.get(name_hash, {}) | { title: deepcopy(result) }

        index.sort(key=lambda j: j["ended_at"])
        index = index[::-1]

        with open("_index.json", "w") as f:
            json.dump(index, f, ensure_ascii=False, indent=4)

        if os.path.exists("history"):
            shutil.rmtree("history")

        os.mkdir("history")

        for (name_hash, history) in case_history.items():
            history = [(title, result) for (title, result) in history.items()]
            history.sort(key=lambda x: timestamp_by_title_map[x[0]])
            history = { title: result | { "seq": i, "ended_at": timestamp_by_title_map[title] } for (i, (title, result)) in enumerate(history) }

            # name: "end_to_end dummy"
            # name_without_args: "end_to_end"
            name = name_by_name_hash_map[name_hash]
            name_without_args = [n for n in name.split(" ")][0]

            if name_without_args not in description_by_name_map:
                print(f"Warning: {name} is not in `description_by_name_map`. Probably because it's an old test and renamed.")
                description = ""

            else:
                description = description_by_name_map[name_without_args]

            with open(os.path.join("history", name_hash + ".json"), "w") as f:
                f.write(json.dumps({
                    "meta": {
                        "name": name,
                        "description": description,
                    },
                    "tests": history,
                    "result": {
                        "total": len(history),
                        "complete": len(history),
                        "pass": len([h for h in history.values() if h["pass"]]),
                        "fail": len([h for h in history.values() if not h["pass"]]),
                        "remaining": 0,
                    },
                }, ensure_ascii=False, indent=4))

    # You should run this in `ragithub/test-results/` and `../../ragit` must exist.
    elif command == "fetch_git_info":
        subprocess.run(["git", "-C", "../../ragit", "pull"])
        log = subprocess.run(["git", "-C", "../../ragit", "log", "--pretty=%h<|delim|>%an<|delim|>%ae<|delim|>%at<|delim|>%s<|delim|>%b<|delim|>%p<|end-of-commit|>", "--abbrev=9"], capture_output=True, text=True).stdout
        commits = [commit.strip() for commit in log.split("<|end-of-commit|>") if commit.strip() != ""]
        commits = [
            {
                "hash": (s := commit.split("<|delim|>"))[0],
                "author_name": s[1],
                "author_email": s[2],
                "timestamp": s[3],
                "title": s[4],
                "message": s[5],
            } for commit in commits
        ]

        if not os.path.exists("commits"):
            os.mkdir("commits")

        for commit in commits:
            hash = commit["hash"]
            directory = os.path.join("commits", hash[:2])

            if not os.path.exists(directory):
                os.mkdir(directory)

            path = os.path.join(directory, hash[2:] + ".json")

            with open(path, "w") as f:
                f.write(json.dumps(commit, ensure_ascii=False, indent=4))

    else:
        print(help_message)
