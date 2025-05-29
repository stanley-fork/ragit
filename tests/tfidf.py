import json
import re
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

def parse_tfidf_output(args: list[str], extra_check: bool = True) -> int:
    output = cargo_run(["tfidf"] + args, stdout=True)
    result = None

    for line in output.split("\n"):
        if (r := re.match(r"^found\s(\d+)\sresults$", line)) is not None:
            result = int(r.group(1))
            break

    if extra_check:
        output = cargo_run(["tfidf", "--json"] + args, stdout=True)
        output = len(json.loads(output.strip()))
        assert result == output

    if result is not None:
        return result

    else:
        raise Exception("no result found")

lorem_ipsum1 = 'Suspendisse scelerisque accumsan gravida. Etiam nec viverra tortor. Praesent neque magna, fringilla id volutpat id, iaculis vitae risus. Aliquam vitae massa id diam ornare malesuada. Suspendisse accumsan erat non lacus placerat euismod. Suspendisse rutrum condimentum nibh, vitae fringilla lorem vulputate at. Vivamus pulvinar nisl eros, mattis suscipit erat consectetur at. Duis condimentum suscipit venenatis. Sed eu velit gravida, efficitur nisl ut, luctus lectus. Aliquam commodo commodo. Fusce blandit lobortis urna sit amet scelerisque. Fusce ut leo lorem. Vestibulum interdum euismod egestas. Maecenas sed sem metus.'
lorem_ipsum2 = 'In eget sem nisl. Nam convallis nunc leo, at venenatis turpis maximus a. Proin id nisi in arcu elementum ultrices. Duis aliquam nisi odio, ut gravida mi volutpat non. Pellentesque tincidunt sollicitudin tellus nec suscipit. Pellentesque non odio porttitor, eleifend erat eget, tempor erat. Aenean ut metus gravida, accumsan nibh vel, ultricies est. Duis nec mi vel purus laoreet elementum. Fusce convallis imperdiet diam, vitae ullamcorper enim ornare et.'

def tfidf():
    goto_root()
    mk_and_cd_tmp_dir()
    write_string("lorem_ipsum1.txt", lorem_ipsum1)
    write_string("lorem_ipsum2.txt", lorem_ipsum2)
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["check"])

    # step 1: tfidf without any chunk
    cargo_run(["tfidf", "hello, world"])

    # step 2: tfidf with only 1 chunk
    cargo_run(["add", "lorem_ipsum1.txt"])
    cargo_run(["build"])
    assert "lorem_ipsum1.txt" not in cargo_run(["tfidf", "hello, world"], stdout=True)
    assert "lorem_ipsum1.txt" in cargo_run(["tfidf", "Praesent neque magna"], stdout=True)

    # step 3: tfidf with multiple chunks
    cargo_run(["add", "lorem_ipsum2.txt"])
    cargo_run(["build"])
    assert "lorem_ipsum1.txt" in cargo_run(["tfidf", "Praesent neque magna"], stdout=True)
    assert "lorem_ipsum2.txt" not in cargo_run(["tfidf", "Praesent neque magna"], stdout=True)
    assert "lorem_ipsum1.txt" not in cargo_run(["tfidf", "Pellentesque tincidunt"], stdout=True)
    assert "lorem_ipsum2.txt" in cargo_run(["tfidf", "Pellentesque tincidunt"], stdout=True)

    # step 4: tfidf on korean
    write_string("korean.txt", "나는 비빔인간입니다.")
    cargo_run(["add", "korean.txt"])
    cargo_run(["build"])
    cargo_run(["check"])
    assert "korean.txt" in cargo_run(["tfidf", "비빔인간"], stdout=True)
    assert "lorem_ipsum1.txt" not in cargo_run(["tfidf", "비빔인간"], stdout=True)
    assert "korean.txt" not in cargo_run(["tfidf", "Pellentesque tincidunt"], stdout=True)

    # step 5: `--limit` option
    for i in range(20):
        write_string(f"{i}.txt", "unique word")
        cargo_run(["add", f"{i}.txt"])

    cargo_run(["build"])

    for _ in range(2):
        assert parse_tfidf_output(["--limit=10", "unique"]) == 10
        assert parse_tfidf_output(["--limit=20", "unique"]) == 20
        assert parse_tfidf_output(["unique", "--limit=15"]) == 15
        assert parse_tfidf_output(["unique", "--limit", "15"]) == 15
        assert parse_tfidf_output(["--limit", "15", "unique"]) == 15
        cargo_run(["ii-build"])
        cargo_run(["check"])
