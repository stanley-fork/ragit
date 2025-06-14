import json
import os
import shutil
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    read_string,
    write_string,
)

def pdl(test_model: str):
    # many other tests requires `test_model != "dummy"` but they don't assert
    # that because dummy models will lead to test failures
    # but in this test, if `test_model` is set to dummy, it might change the result
    # of the test. so it has to be asserted
    assert test_model != "dummy"

    # this test overwrites some files in `~/.config/ragit`. You have to run
    # this test in an isolated environment.
    home_dir = os.path.expanduser("~")
    global_config_dir = os.path.join(home_dir, ".config", "ragit")
    assert not os.path.exists(global_config_dir), "~/.config/ragit is found. Please run this test in an isolated environment."

    try:
        os.mkdir(global_config_dir)
        goto_root()
        mk_and_cd_tmp_dir()

        # test 1: simple schema and reading local config files
        write_string("test1.pdl", """
    <|schema|>

    { name: str, age: int }

    <|system|>

    You'll be given a short story with a character. Your job is to extract a name and age of the character.

    Your response must be a json object, with 2 keys: "name" and "age". "name" is a string and "age" is an integer.

    <|user|>

    There lives a man named ragit. He is 26 years old.
    """)

        # if there's no knowledge-base, you have to specify model name
        assert cargo_run(["pdl", "test1.pdl"], check=False) != 0

        # if there's knowledge-base and `--model` is not set, it reads
        # configuration of the knowledge-base
        cargo_run(["init"])
        cargo_run(["config", "--set", "model", "dummy"])

        # when schema is set, `rag pdl` always outputs a json value
        # since we're using a dummy model, the schema validation would fail
        # and it would dump `null`
        assert json.loads(cargo_run(["pdl", "test1.pdl"], stdout=True)) == None

        # now let's run the actual test
        result = json.loads(cargo_run(["pdl", "test1.pdl", "--model", test_model], stdout=True))
        assert result == { "name": "ragit", "age": 26 }

        # test 2: simple image and reading global config files

        # let's keep it for later use
        shutil.copyfile(".ragit/models.json", "models.json")

        shutil.rmtree(".ragit")
        shutil.copyfile("../tests/images/hello_world.webp", "sample.webp")

        write_string("test2.pdl", """
    <|user|>

    I have an image of a wooden plank. There's something written on it... What does it say?

    <|media(sample.webp)|>
    """)

        # if there's no knowledge-base, you have to specify model name
        assert cargo_run(["pdl", "test2.pdl"], check=False) != 0

        # if there's a global models.json, but no global api config, it'll still fail
        shutil.copyfile("models.json", os.path.join(global_config_dir, "models.json"))
        assert cargo_run(["pdl", "test2.pdl"], check=False) != 0

        # if we have both global api config and global models.json, it'll run
        write_string(
            os.path.join(global_config_dir, "api.json"),
            "{ \"model\": \"dummy\" }",
        )
        assert cargo_run(["pdl", "test2.pdl"], stdout=True).strip() == "dummy"

        # now let's run the actual test
        result = cargo_run(["pdl", "test2.pdl", "--model", test_model], stdout=True).lower()
        assert "hello" in result
        assert "world" in result

        # test 3: another simple schema and logging
        write_string("test3.pdl", """
<|user|>

Say something
""")
        # test 3.1: without schema
        cargo_run(["pdl", "test3.pdl", "--model=dummy", "--log=log1"])
        log_file = [f for f in os.listdir("log1") if f.endswith(".pdl")]
        assert len(log_file) == 1
        log_file = os.path.join("log1", log_file[0])

        # since there's no schema, there's no failure!
        assert read_string(log_file).count("<|Assistant|>") == 1

        # test 3.2: with schema
        cargo_run(["pdl", "test3.pdl", "--model=dummy", "--schema=str { min: 100 }", "--log=log2"])
        log_file = [f for f in os.listdir("log2") if f.endswith(".pdl")]
        assert len(log_file) == 1
        log_file = os.path.join("log2", log_file[0])

        # this time, pdl asks the dummy model to output at least 100 characters
        # since the dummy model can only speak "dummy", it'll fail multiple times
        assert read_string(log_file).count("<|Assistant|>") > 1
        assert "at least 100 characters" in read_string(log_file)

        # test 4: test tera engine
        write_string("test4.pdl", """
<|schema|>

integer

<|user|>

Below is the list of the customers

- {{customer1.name}}: {{customer1.age}} years old
- {{customer2.name}}: {{customer2.age}} years old
- {{customer3.name}}: {{customer3.age}} years old

How old is {{customer1.name}}?

<|assistant|>
""")
        context = {
            "customer1": { "name": "Bae", "age": 29 },
            "customer2": { "name": "Hyun", "age": 35 },
            "customer3": { "name": "Sol", "age": 16 },
        }
        broken_context = {
            "customer1": [],
        }

        with open("context.json", "w") as f:
            f.write(json.dumps(context))

        with open("broken_context.json", "w") as f:
            f.write(json.dumps(broken_context))

        assert int(cargo_run(["pdl", "test4.pdl", "--model", test_model, "--context=context.json"], stdout=True)) == context["customer1"]["age"]

        # If the context is broken, `--strict` will refuse to run and no `--strict` will run it with the broken context
        cargo_run(["pdl", "test4.pdl", "--model", "dummy", "--context=broken_context.json"])
        assert cargo_run(["pdl", "test4.pdl", "--strict", "--model", "dummy", "--context=broken_context.json"], check=False) != 0

    finally:
        shutil.rmtree(global_config_dir)
