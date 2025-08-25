import json
import os
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

def ls_queries():
    goto_root()
    mk_and_cd_tmp_dir()

    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])

    # I want to make sure that the queries are sorted by timestamp. But if I do multiple
    # queries in a second, they will have the same timestamp and I cannot compare them.
    cargo_run(["config", "--set", "sleep_after_llm_call", "500"])

    write_string("sample.md", "Hello, World!")
    cargo_run(["add", "sample.md"])
    cargo_run(["build"])
    kb_uid1 = cargo_run(["uid"], stdout=True).strip()

    # no queries yet
    assert len(json.loads(cargo_run(["ls-queries", "--json"], stdout=True))) == 0
    assert json.loads(cargo_run(["ls-queries", "--stat-only", "--json"], stdout=True))["queries"] == 0

    cargo_run(["query", "Hello, World?"])

    # a query history does not change the uid of the knowledge-base
    kb_uid2 = cargo_run(["uid"], stdout=True).strip()
    assert kb_uid1 == kb_uid2

    # now we have 1 query
    assert len(json.loads(cargo_run(["ls-queries", "--json"], stdout=True))) == 1
    assert json.loads(cargo_run(["ls-queries", "--stat-only", "--json"], stdout=True))["queries"] == 1

    query_contents = json.loads(cargo_run(["ls-queries", "--content-only", "--json"], stdout=True))
    assert len(query_contents) == 1
    query1_content = query_contents[0]

    # let's make sure that the content is correct
    assert query1_content[0]["content"] == "Hello, World?"

    # 1 user turn, 1 assistant turn
    assert len(query1_content) == 2

    query_uid1 = json.loads(cargo_run(["ls-queries", "--uid-only", "--json"], stdout=True))[0]

    # an agent mode also creates a query history
    cargo_run(["query", "--agent", "Hi there! You are a dummy agent"])

    # now we have 2 queries
    assert len(json.loads(cargo_run(["ls-queries", "--json"], stdout=True))) == 2
    assert json.loads(cargo_run(["ls-queries", "--stat-only", "--json"], stdout=True))["queries"] == 2

    # It's ordered by timestamp, in a descending order
    query_uid2 = json.loads(cargo_run(["ls-queries", "--uid-only", "--json"], stdout=True))[0]
    assert query_uid1 != query_uid2

    # We can continue a previous conversation with `--continue` option.
    cargo_run(["query", "--continue", query_uid1, "Let's continue the conversation!"])

    # We still have 2 queries
    assert len(json.loads(cargo_run(["ls-queries", "--json"], stdout=True))) == 2
    assert json.loads(cargo_run(["ls-queries", "--stat-only", "--json"], stdout=True))["queries"] == 2

    # 2 user turns, 2 assistant turns
    query1_content = json.loads(cargo_run(["ls-queries", query_uid1, "--content-only", "--json"], stdout=True))[0]
    assert len(query1_content) == 4

    # We can even `--continue` with an agent!
    cargo_run(["query", "--agent", "--continue", query_uid1, "Let's continue the conversation! But now you're an agent."])

    # We still have 2 queries
    assert len(json.loads(cargo_run(["ls-queries", "--json"], stdout=True))) == 2
    assert json.loads(cargo_run(["ls-queries", "--stat-only", "--json"], stdout=True))["queries"] == 2

    # 3 user turns, 3 assistant turns
    query1_content = json.loads(cargo_run(["ls-queries", query_uid1, "--content-only", "--json"], stdout=True))[0]
    assert len(query1_content) == 6

    # let's create archives, with and without query histories
    cargo_run(["archive", "-o", "../with-queries.ar", "--queries"])
    cargo_run(["archive", "-o", "../without-queries.ar", "--no-queries"])

    os.chdir("..")
    cargo_run(["extract", "with-queries.ar", "-o", "with-queries"])
    cargo_run(["extract", "without-queries.ar", "-o", "without-queries"])

    os.chdir("with-queries")
    cargo_run(["check"])

    # query history does not change the uid of the knowledge-base
    assert cargo_run(["uid"], stdout=True).strip() == kb_uid1

    # all the query history must be there (and query uid must be the same)
    assert len(json.loads(cargo_run(["ls-queries", "--json"], stdout=True))) == 2
    assert json.loads(cargo_run(["ls-queries", "--stat-only", "--json"], stdout=True))["queries"] == 2
    query1_content = json.loads(cargo_run(["ls-queries", query_uid1, "--content-only", "--json"], stdout=True))[0]
    assert len(query1_content) == 6

    os.chdir("../without-queries")
    cargo_run(["check"])
    assert cargo_run(["uid"], stdout=True).strip() == kb_uid1

    # There should be no queries
    assert len(json.loads(cargo_run(["ls-queries", "--json"], stdout=True))) == 0
    assert json.loads(cargo_run(["ls-queries", "--stat-only", "--json"], stdout=True))["queries"] == 0

    # TODO: create a lot of query histories, and check if `ls-queries` sort them by timestamp
    #       I have to check: with/without uid prefix, with/without --uid-only, with/without --json
