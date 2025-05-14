import json
import os
import shutil
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

# `broken-svg.svg` contains this string
magic_string = "ragit-svg-test"

def svg(test_model: str):
    goto_root()
    mk_and_cd_tmp_dir()
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])

    # (name, is_valid)
    svg_files = [
        ("textbox.svg", True),
        ("circle.svg", True),
        ("red-circle.svg", True),
        ("broken-svg.svg", False),
    ]

    for svg_file, _ in svg_files:
        shutil.copyfile(f"../tests/svgs/{svg_file}", svg_file)

    # This svg file is so broken that it's not even a text file
    shutil.copyfile("../tests/images/empty.webp", "broken-non-text.svg")
    svg_files.append(("broken-non-text.svg", False))

    for svg_file, _ in svg_files:
        write_string(svg_to_md(svg_file), f"This is an svg file: ![]({svg_file}) That was an svg file")

    valid_files = [svg_file for svg_file, is_valid in svg_files if is_valid]
    broken_files = [svg_file for svg_file, is_valid in svg_files if not is_valid]

    # step 1: image reader, with a strict file reader
    cargo_run(["add", *[svg_file for svg_file, _ in svg_files]])
    cargo_run(["config", "--set", "strict_file_reader", "true"])
    cargo_run(["build"])
    stat = json.loads(cargo_run(["ls-files", "--stat-only", "--json"], stdout=True))

    # cannot process the broken svgs
    assert stat["staged files"] == len(broken_files)
    assert stat["processed files"] == len(valid_files)

    cargo_run(["remove", "--all"])

    # step 2: image reader, without a strict file reader
    cargo_run(["add", *[svg_file for svg_file, _ in svg_files]])
    cargo_run(["config", "--set", "strict_file_reader", "false"])
    cargo_run(["build"])
    stat = json.loads(cargo_run(["ls-files", "--stat-only", "--json"], stdout=True))

    # the loose file reader treats broken svg files as text files
    assert stat["processed files"] == len(svg_files)
    assert magic_string in cargo_run(["cat-file", "broken-svg.svg"], stdout=True)

    cargo_run(["remove", "--all"])

    # step 3: markdown reader, with a strict file reader
    cargo_run(["add", *[svg_to_md(svg_file) for svg_file, _ in svg_files]])
    cargo_run(["config", "--set", "strict_file_reader", "true"])
    cargo_run(["build"])
    stat = json.loads(cargo_run(["ls-files", "--stat-only", "--json"], stdout=True))

    # cannot process the markdown files with broken svgs
    assert stat["staged files"] == len(broken_files)
    assert stat["processed files"] == len(valid_files)

    cargo_run(["remove", "--all"])

    # step 4: markdow reader, without a strict file reader
    cargo_run(["add", *[svg_to_md(svg_file) for svg_file, _ in svg_files]])
    cargo_run(["config", "--set", "strict_file_reader", "false"])
    cargo_run(["build"])
    stat = json.loads(cargo_run(["ls-files", "--stat-only", "--json"], stdout=True))

    # Valid svg files are replaced with `img_{uid}`, while broken ones are not changed
    # and left `![](broken.svg)`.
    for svg_file, is_valid in svg_files:
        cat_file_result = cargo_run(["cat-file", svg_to_md(svg_file)], stdout=True)

        if is_valid:
            assert svg_file not in cat_file_result

        else:
            assert svg_file in cat_file_result

    # step 5: check if the converted png files are correct
    #
    # Ragit internally converts svg files to png files and
    # feed the png files to LLM.
    cargo_run(["config", "--set", "model", test_model])
    cargo_run(["gc", "--images"])
    assert len(json.loads(cargo_run(["ls-images", "--json"], stdout=True))) == len(valid_files)

    # find the converted png files
    for svg_file in valid_files:
        chunks = json.loads(cargo_run(["ls-chunks", svg_to_md(svg_file), "--json"], stdout=True))
        image_uid = chunks[0]["images"][0]
        image_path = os.path.join(
            ".ragit",
            "images",
            image_uid[:2],
            image_uid[2:] + ".png",
        )
        shutil.copyfile(image_path, svg_to_png(svg_file))

    # now we have `textbox.png`, `circle.png`, `red-circle.png`

    write_string("test1.pdl", """
<|user|>

According to this image, what is the name of the project?

<|media(textbox.png)|>
""")
    assert "ragit" in cargo_run(["pdl", "test1.pdl"], stdout=True).lower()

    write_string("test2.pdl", """
<|user|>

What shape do you see in this image? It's either a circle, rectangle, or a triangle.

<|media(circle.png)|>

Answer with a single word. DO NOT say any other word.
""")
    response = cargo_run(["pdl", "test2.pdl"], stdout=True).lower()
    assert "circle" in response
    assert "rect" not in response
    assert "triangle" not in response

    response = write_string("test3.pdl", """
<|user|>

What color is the circle? It's either red, green, or blue.

<|media(red-circle.png)|>

Answer with a single word. DO NOT say any other word.
""")
    response = cargo_run(["pdl", "test3.pdl"], stdout=True).lower()
    assert "red" in response
    assert "green" not in response
    assert "blue" not in response

def svg_to_md(path: str) -> str:
    return path[:-4] + ".md"

def svg_to_png(path: str) -> str:
    return path[:-4] + ".png"
