import json
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, write_string

def parse_audit_output(category: str) -> int:  # tokens
    result1 = cargo_run(["audit", "--json", "--only-tokens"] + (["-c=" + category] if category is not None else []), stdout=True)
    result2 = cargo_run(["audit", "--json", "--only-tokens"], stdout=True)
    result1 = json.loads(result1)
    result2 = json.loads(result2)

    assert result1 == result2[category]
    return result1["total tokens"]

def audit(test_model: str):
    goto_root()
    mk_and_cd_tmp_dir()
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", test_model])
    cargo_run(["config", "--set", "dump_api_usage", "false"])
    cargo_run(["query", "Why is the sky blue?"])

    # nothing's dumped
    assert parse_audit_output("total") == 0

    cargo_run(["config", "--set", "dump_api_usage", "true"])
    cargo_run(["query", "Why is the sky blue?"])

    # there's no chunk, so it always uses `raw_request.pdl`
    assert parse_audit_output("raw_request") > 0
    assert parse_audit_output("answer_query_with_chunks") == 0
    assert parse_audit_output("create_chunk_from") == 0

    write_string("why_sky_is_blue.txt", "The sky appears blue because of a phenomenon called Rayleigh scattering.")
    cargo_run(["add", "why_sky_is_blue.txt"])
    cargo_run(["build"])
    cargo_run(["query", "Why is the sky blue?"])

    assert parse_audit_output("answer_query_with_chunks") > 0
    assert parse_audit_output("create_chunk_from") > 0

    cargo_run(["gc", "--audit"])

    assert parse_audit_output("total") == 0
    assert parse_audit_output("answer_query_with_chunks") == 0
    assert parse_audit_output("create_chunk_from") == 0
