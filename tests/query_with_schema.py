import json
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
)

prompt = '''
I'll give you a short story. There're 2 characters in the story. Please extract the names of the characters. You have to

1. Give me a json array. It's an array of strings, where each string is the name of the characters.
2. The json array must be in a fenced code block. A code fence is a sequence of three backtick characters (`). It's markdown syntax.

Story:

There lived a man named ragit. He's father is bae. They both lived happily.
'''

# NOTE: the model has to be smart enough to solve the problem.
def query_with_schema(test_model: str):
    goto_root()
    mk_and_cd_tmp_dir()
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", test_model])
    cargo_run(["config", "--set", "dump_log", "true"])

    # NOTE: you can query in an empty knowledge-base! It'll query without chunks
    output1 = cargo_run(["query", prompt], stdout=True)
    output2 = cargo_run(["query", prompt, "--schema", "code"], stdout=True)
    output3 = cargo_run(["query", prompt, "--schema", "[str]"], stdout=True)

    try:
        json.loads(output1)
        assert False, "`json.loads()` is supposed to fail! `output1` is not a valid json!!"

    except:
        pass

    output2 = [o.lower() for o in json.loads(output2)]
    output3 = [o.lower() for o in json.loads(output3)]

    assert set(output2) == { "ragit", "bae" }
    assert set(output3) == { "ragit", "bae" }
