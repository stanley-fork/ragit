import json
import os
from random import randint
import re
import shutil
from utils import (
    cargo_run,
    count_files,
    count_images,
    goto_root,
    mk_and_cd_tmp_dir,
    rand_word,
    write_string,
)

def create_doc_with_magic_words(magic_word1: str, magic_word2: str) -> str:
    return "\n".join([magic_word1] + ["delim" for _ in range(randint(150, 750))] + [magic_word2])

# TODO: test with empty files
def ls():
    while True:
        goto_root()
        mk_and_cd_tmp_dir()
        cargo_run(["init"])
        cargo_run(["config", "--set", "model", "dummy"])
        cargo_run(["config", "--set", "chunk_size", "512"])
        cargo_run(["config", "--set", "slide_len", "128"])
        magic_words = []
        file_names = []
        magic_words_map = {}  # magic_word -> file_name
        file_map = {}  # file_uid -> file_name
        file_map_rev = {}  # file_name -> file_uid
        uid_map = {}  # chunk_uid -> file_uid
        file_image_map = {}  # file_name -> image_name
        image_uid_map = {}  # image_name -> image_uid

        for i in range(8):
            file_name = f"sample_file_{i}.txt"
            magic_word1, magic_word2 = rand_word().lower(), rand_word().lower()  # tfidfs' are case-insensitive, and Python's `in` operator is case-sensitive
            write_string(file_name, create_doc_with_magic_words(magic_word1, magic_word2))
            magic_words += [magic_word1, magic_word2]
            file_names.append(file_name)
            magic_words_map[magic_word1] = file_name
            magic_words_map[magic_word2] = file_name
            cargo_run(["add", file_name])

        write_string("image1.md", "![image](sample1.png)")
        write_string("image2.md", "![image](sample2.jpg)")
        write_string("no_image.md", "This is not an image.")
        shutil.copyfile("../tests/images/empty.png", "sample1.png")
        shutil.copyfile("../tests/images/empty.jpg", "sample2.jpg")
        file_names += ["image1.md", "image2.md", "no_image.md"]
        cargo_run(["add", "image1.md", "image2.md", "no_image.md"])
        file_image_map["image1.md"] = "sample1.png"
        file_image_map["image2.md"] = "sample2.jpg"
        assert count_images() == 0

        cargo_run(["build"])
        cargo_run(["check"])
        assert count_images() == 2

        # `rag ls-terms` dumps stemmed terms. For example, if a magic word is `ragagapic`, the tfidf engine would turn it into `ragagap`.
        print("step 0: reconcile ragit's stemmer and python")
        terms = json.loads(cargo_run(["ls-terms", "--term-only", "--json"], stdout=True))
        terms = [term for term in terms if term != "delim"]

        # I'm too lazy to create a map. Let's just initialize over and over until there's no stemmer issue
        if all([magic_word in terms for magic_word in magic_words]):
            break

    print("step 1: check if `tfidf` command can retrieve the magic words (with and without ii-build)")

    for _ in range(2):
        for magic_word in magic_words:
            tfidf_result = cargo_run(["tfidf", magic_word], stdout=True)

            for file_name in file_names:
                if magic_words_map[magic_word] == file_name:
                    assert file_name in tfidf_result

                else:
                    assert file_name not in tfidf_result

        cargo_run(["ii-build"])
        cargo_run(["check"])

    print("step 2: construct `file_map` from `ls-files`")
    ls_files_result = cargo_run(["ls-files", "--abbrev=64"], stdout=True).split("-----")

    for file in ls_files_result:
        if "uid: " not in file:
            continue

        lines = file.split("\n")

        for line in lines:
            if (r := re.match(r"^uid\:\s([a-f0-9]{64})$", line)) is not None:
                file_uid = r.group(1)

            if (r := re.match(r"^name\:\s(.+)$", line)) is not None:
                file_name = r.group(1)

        file_map_rev[file_name] = file_uid
        file_map[file_uid] = file_name

    print("step 3: construct `uid_map` from `ls-chunks`")
    ls_chunks_result = cargo_run(["ls-chunks", "--abbrev=64"], stdout=True).split(" chunk of ")

    for chunk in ls_chunks_result:
        if "uid: " not in chunk:
            continue

        lines = chunk.split("\n")
        file_name = lines[0]

        for line in lines:
            if (r := re.match(r"^uid\:\s([a-f0-9]{64})$", line)) is not None:
                chunk_uid = r.group(1)
                uid_map[chunk_uid] = file_map_rev[file_name]
                break

    print("step 4: `ls-chunks <CHUNK-UID>`")

    for chunk_uid, file_uid in uid_map.items():
        for search_key in [chunk_uid, chunk_uid[:8]]:  # prefix search
            ls_chunks_result = cargo_run(["ls-chunks", search_key, "--abbrev=64"], stdout=True)
            file_name = file_map[file_uid]
            assert chunk_uid in ls_chunks_result
            assert file_name in ls_chunks_result

            for chunk_uid_ in uid_map.keys():
                if chunk_uid != chunk_uid_:
                    assert chunk_uid_ not in ls_chunks_result

            for file_name_ in file_names:
                if file_name != file_name_:
                    assert file_name_ not in ls_chunks_result

    print("step 5: `ls-chunks <FILE-UID>`")

    for file_uid, file_name in file_map.items():
        for search_key in [file_uid, file_uid[:8], file_name]:
            ls_chunks_result = cargo_run(["ls-chunks", search_key, "--abbrev=64"], stdout=True)
            assert file_name in ls_chunks_result

            for chunk_uid in uid_map.keys():
                if file_uid != uid_map[chunk_uid]:
                    assert chunk_uid not in ls_chunks_result

                else:
                    assert chunk_uid in ls_chunks_result

            for file_name_ in file_names:
                if file_name != file_name_:
                    assert file_name_ not in ls_chunks_result

    print("step 6: `ls-files <FILE-UID>`")

    for file_uid, file_name in file_map.items():
        for search_key in [file_uid, file_uid[:8], file_name]:
            ls_files_result = cargo_run(["ls-files", search_key, "--abbrev=64"], stdout=True)
            assert file_name in ls_files_result
            assert file_uid in ls_files_result

            for file_uid_ in file_map.keys():
                file_name_ = file_map[file_uid_]

                if file_uid != file_uid_:
                    assert file_uid_ not in ls_files_result

                if file_name != file_name_:
                    assert file_name_ not in ls_files_result

            for line in ls_files_result.split("\n"):
                if (r := re.match(r"^chunks\:\s(\d+)$", line)) is not None:
                    assert int(r.group(1)) == len([chunk_uid for chunk_uid, file_uid_ in uid_map.items() if file_uid == file_uid_])

    print("step 7: `ls-terms <CHUNK-UID>`")
    appeared_magic_word = set()

    for chunk_uid, file_uid in uid_map.items():
        for search_key in [chunk_uid, chunk_uid[:8]]:
            ls_terms_result = cargo_run(["ls-terms", search_key], stdout=True)
            file_name = file_map[file_uid]

            for magic_word in magic_words:
                file_name_ = magic_words_map[magic_word]

                if file_name != file_name_:
                    assert magic_word not in ls_terms_result

                if magic_word in ls_terms_result:
                    appeared_magic_word.add(magic_word)
                    assert file_name == file_name_

    assert len(appeared_magic_word) == len(magic_words_map)

    print("step 8: `ls-terms <FILE-UID>`")

    for file_uid, file_name in file_map.items():
        for search_key in [file_uid, file_uid[:8], file_name]:
            ls_terms_result = cargo_run(["ls-terms", search_key], stdout=True)

            for magic_word in magic_words:
                file_name_ = magic_words_map[magic_word]

                if file_name == file_name_:
                    assert magic_word in ls_terms_result

                else:
                    assert magic_word not in ls_terms_result

    print("step 9: file path query in other directories")
    os.mkdir("dir")
    os.chdir("dir")
    ls_files_result = cargo_run(["ls-files", f"../{file_names[0]}", "--abbrev=64"], stdout=True)
    assert file_map_rev[file_names[0]] in ls_files_result
    assert file_map_rev[file_names[1]] not in ls_files_result

    print("step 9.1: see if `ls-files ..` works")
    ls_files_result = cargo_run(["ls-files", ".."], stdout=True)

    for file_name in file_names:
        assert file_name in ls_files_result

    os.chdir("..")
    os.rmdir("dir")

    print("step 10: construct `image_uid_map` from `ls-images`")

    for file_name, image_name in file_image_map.items():
        ls_images_result = cargo_run(["ls-images", file_name, "--abbrev=64"], stdout=True)

        for line in ls_images_result.split("\n"):
            if (r := re.match(r"uid\:\s([a-f0-9]{64})", line)) is not None:
                image_uid_map[image_name] = r.group(1)
                break

    assert len(image_uid_map) == len(file_image_map)

    print("step 11: `ls-images <FILE-UID>`")

    for file_uid, file_name in file_map.items():
        for search_key in [file_uid, file_uid[:8], file_name]:
            if file_name in file_image_map:
                ls_images_result = cargo_run(["ls-images", search_key, "--abbrev=64"], stdout=True)
                image_uid = image_uid_map[file_image_map[file_name]]
                assert image_uid in ls_images_result

                for image_uid_ in image_uid_map.values():
                    if image_uid != image_uid_:
                        assert image_uid_ not in ls_images_result

            else:
                assert cargo_run(["ls-images", search_key], check=False) != 0

    print("step 12: `ls-images <CHUNK-UID>`")

    for chunk_uid, file_uid in uid_map.items():
        file_name = file_map[file_uid]

        for search_key in [chunk_uid, chunk_uid[:8]]:
            if file_name in file_image_map:
                ls_images_result = cargo_run(["ls-images", search_key, "--abbrev=64"], stdout=True)
                image_uid = image_uid_map[file_image_map[file_name]]
                assert image_uid in ls_images_result

            else:
                assert cargo_run(["ls-images", search_key], check=False) != 0

    print("step 13: `ls-images <IMAGE-UID>`")
    invalid_uid = "0123abcd" * 8
    assert cargo_run(["ls-images", invalid_uid], check=False) != 0
    assert cargo_run(["ls-images", invalid_uid[:8]], check=False) != 0

    for image_uid in image_uid_map.values():
        for search_key in [image_uid, image_uid[:8]]:
            ls_images_result = cargo_run(["ls-images", search_key], stdout=True)
            assert "1 image" in ls_images_result

    print("step 14: make sure that `--json` option makes the output a valid json")
    sample_chunk_uid = list(uid_map.keys())[0]
    sample_file_uid = list(uid_map.values())[0]

    for flag in [
        [],
        ["--stat-only"],
        ["--uid-only"],
    ]:
        for query in [
            [],
            [sample_chunk_uid],
            [sample_file_uid],
        ]:
            ls_chunks_result = cargo_run(["ls-chunks", "--json"] + flag + query, stdout=True)
            json.loads(ls_chunks_result)

    print("step 15: multiple args for `ls-chunks`")
    chunk1, chunk2, chunk3 = list(uid_map.keys())[:3]
    file1, file2, file3 = file_map[uid_map[chunk1]], file_map[uid_map[chunk2]], file_map[uid_map[chunk3]]
    ls_chunks_result = cargo_run(["ls-chunks", chunk1, chunk2], stdout=True)
    assert file1 in ls_chunks_result
    assert file2 in ls_chunks_result
    assert file3 not in ls_chunks_result

    print("step 16: directory path query")
    cargo_run(["rm", file1, file2, file3])

    os.mkdir("dir")
    os.chdir("dir")
    shutil.copyfile(f"../{file1}", file1)
    shutil.copyfile(f"../{file2}", file2)
    cargo_run(["add", "."])
    os.chdir("..")
    cargo_run(["build"])
    cargo_run(["check"])

    assert cargo_run(["ls-files", file1], check=False) != 0
    assert count_files(["dir"]) == (2, 0, 2)

    ls_files_result = cargo_run(["ls-files", "dir"], stdout=True)

    assert file1 in ls_files_result
    assert file2 in ls_files_result
    assert file3 not in ls_files_result

    print("step 17: `ls-files --staged` and `ls-files --processed`")

    for i in range(2):
        total, staged, processed = count_files()
        staged_files = json.loads(cargo_run(["ls-files", "--staged", "--json", "--name-only"], stdout=True))
        processed_files = json.loads(cargo_run(["ls-files", "--processed", "--json", "--name-only"], stdout=True))
        assert staged == len(staged_files)
        assert processed == len(processed_files)

        # let's test it with/without a staged file
        if i == 0:
            write_string("tmp-file", "abcdefg")
            cargo_run(["add", "tmp-file"])

    # It returns an error if `ls-files` has no files to list.
    #
    # TODO: I'm not sure whether it should return an empty list or an error. Both make perfect sense.
    #       For example, `git checkout`, `git log` and many other commands that takes a commit hash
    #       as an input fails if there's no commit with the hash. But `git ls-files` doesn't raise error
    #       but dumps an empty string when there's no files to list.
    assert len(json.loads(cargo_run(["ls-files", "tmp-file", "--staged", "--json", "--name-only"], stdout=True))) == 1
    assert cargo_run(["ls-files", "tmp-file", "--processed", "--json", "--name-only"], check=False) != 0
    assert cargo_run(["ls-files", processed_files[0], "--staged", "--json", "--name-only"], check=False) != 0
    assert len(json.loads(cargo_run(["ls-files", processed_files[0], "--processed", "--json", "--name-only"], stdout=True))) == 1
