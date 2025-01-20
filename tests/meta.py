import json
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir

def meta():
    goto_root()
    mk_and_cd_tmp_dir()
    cargo_run(["init"])
    metadata = {}

    for command, args in [
        ("get", ["key1"]),
        ("set", ["key1", "value1"]),
        ("get", ["key1"]),
        ("set", ["key1", "value2"]),
        ("get", ["key1"]),
        ("set", ["key2", "value3"]),
        ("remove", ["key1"]),
        ("remove-all", []),
    ]:
        if command == "get":
            key = args[0]

            if key not in metadata:
                assert cargo_run(["meta", "--get", key], check=False) != 0

            else:
                assert metadata[key] == cargo_run(["meta", "--get", key], stdout=True).strip()

        elif command == "set":
            key, value = args
            set_result = cargo_run(["meta", "--set", key, value], stdout=True)

            if key in metadata:
                prev_value = metadata[key]
                assert prev_value in set_result and value in set_result

            else:
                assert value in set_result

            metadata[key] = value

        elif command == "remove":
            key = args[0]

            if key in metadata:
                assert metadata[key] in cargo_run(["meta", "--remove", key], stdout=True)
                metadata.pop(key)

            else:
                assert cargo_run(["meta", "--remove", key], check=False) != 0

        assert json.loads(cargo_run(["meta", "--get-all"], stdout=True)) == metadata
