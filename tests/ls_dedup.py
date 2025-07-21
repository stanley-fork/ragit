import shutil
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    rand_word,
    write_string,
)

# NOTE: It uses magic words to set the prefixes of the uids.
#       I just brute-forced to find the magic words, like mining
#       bitcoins.
#
#       If the magic word doesn't produce the designated prefixes,
#       that's a bug: Uid has to be deterministic regardless of
#       ragit versions.
def ls_dedup():
    goto_root()
    mk_and_cd_tmp_dir()
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    shutil.copyfile("../tests/images/green.png", "image.png")

    # test 1: If an image and a chunk have the same uid prefix,
    #         `ls-images <prefix>` might match both at the same time.
    #         It has to deduplicate the result.
    magic_word = "bvhsbsfe"
    write_string("a.md", "This is an image: ![](image.png) " + magic_word)
    cargo_run(["add", "a.md"])
    cargo_run(["build"])

    image_uid = eval(cargo_run(["ls-images", "--uid-only", "--json"], stdout=True))[0]
    file_uid = eval(cargo_run(["ls-files", "--uid-only", "--json"], stdout=True))[0]
    assert image_uid.startswith("55")
    assert file_uid.startswith("55")

    magic_word = "gnfkrpyvr"
    write_string("b.md", "This is an image: ![](image.png) " + magic_word)
    cargo_run(["add", "b.md"])
    cargo_run(["build"])

    chunk_uid = eval(cargo_run(["ls-chunks", "b.md", "--uid-only", "--json"], stdout=True))[0]
    assert chunk_uid.startswith("55")

    # test 2: If a chunk and a file it belongs to have the same
    #         uid prefix, `ls-chunks <prefix>` might match both
    #         at the same time. It has to deduplicate the result.
    magic_word = "kipuzibj"
    write_string("c.md", "There's no image! " + magic_word)
    cargo_run(["add", "c.md"])
    cargo_run(["build"])

    chunk_uid = eval(cargo_run(["ls-chunks", "c.md", "--uid-only", "--json"], stdout=True))[0]
    file_uid = eval(cargo_run(["ls-files", "c.md", "--uid-only", "--json"], stdout=True))[0]
    assert chunk_uid.startswith("93")
    assert file_uid.startswith("93")

    assert len(eval(cargo_run(["ls-images", "55", "--uid-only", "--json"], stdout=True))) == 1
    assert len(eval(cargo_run(["ls-chunks", "93", "--uid-only", "--json"], stdout=True))) == 1
