import json
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

def csv_reader():
    goto_root()
    mk_and_cd_tmp_dir()

    data = {
        "normal.csv": [
            {"name": "Alice", "age": 30, "city": "New York"},
            {"name": "Bob", "age": 25, "city": "San Francisco"},
        ],
        "empty.csv": [],
        "malformed.csv": [
            {"name": "Alice", "age": 30, "city": "New York"},
            {"name": "Bob", "age": 25},
        ],
    }
    write_string("normal.csv", "name,age,city\nAlice,30,New York\nBob,25,San Francisco\n")
    write_string("empty.csv", "")
    write_string("malformed.csv", "name,age,city\nAlice,30,New York\nBob,25\n")

    cargo_run(["init"])
    cargo_run(["add", "--all"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["config", "--set", "strict_file_reader", "false"])
    cargo_run(["build"])
    cargo_run(["check"])

    for file, data in data.items():
        parsed_data = []
        # csv reader converts the data to jsonl
        stdout = cargo_run(["cat-file", file], stdout=True)

        for line in stdout.split("\n"):
            if not line:
                continue

            parsed_data.append(json.loads(line))

        assert parsed_data == data

    cargo_run(["remove", "normal.csv", "empty.csv", "malformed.csv"])
    cargo_run(["config", "--set", "strict_file_reader", "true"])
    cargo_run(["add", "normal.csv", "empty.csv"])
    cargo_run(["build"])
    cargo_run(["check"])

    cargo_run(["add", "malformed.csv"])
    cargo_run(["build"])

    # failed to process malformed.csv
    assert "malformed.csv" not in json.loads(cargo_run(["ls-files", "--processed", "--name-only", "--json"], stdout=True))
    assert "malformed.csv" in json.loads(cargo_run(["ls-files", "--staged", "--name-only", "--json"], stdout=True))
