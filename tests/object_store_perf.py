# Ragit uses git-like object storage.
#
# 1. It calculates the hash of the object.
# 2. The object is stored in a directory where the first 2 characters of the
#    hash is the name of the directory and the remaining characters are the name of the file.
# 3. For example, if the object's hash is "abcd1234", it's stored at `objects/ab/cd1234`.
#
# My initial thought was: 1) git works this way and git's clever, so this way must also be
#                         clever and 2) it seems like a good idea to separate directories
#                         instead of putting all the object in a single directory.
#
# But I haven't validated my idea. So I wrote a very simple test script that compares git-like
# object store and putting all the objects in a single directory.
#
# It's very simple and naive test. I just want to see one method is roughly as good as the other
# method, or it's dramatically better than the other.

import json
import os
from random import randint
import shutil
import time
from typing import Tuple

def sum_digits(f: str) -> int:
    return sum([eval("0x" + c) for c in f])

def generate_file_and_content() -> Tuple[str, str]:
    r = randint(0, 1 << 36)
    f = f"{r:09x}"
    n = sum_digits(f)

    return f, str(n)

steps = []

def new_step(title: str):
    if steps != []:
        steps[-1]["ended_at"] = time.time()
        steps[-1]["elapsed_ms"] = int((steps[-1]["ended_at"] - steps[-1]["started_at"]) * 1000)
        print(f"Step {len(steps) - 1} took {steps[-1]['elapsed_ms']:,} ms")

    print(f"Step {len(steps)}: {title}")
    steps.append({
        "seq": len(steps),
        "title": title,
        "started_at": time.time(),
    })

if __name__ == "__main__":
    if os.path.exists("git-like"):
        shutil.rmtree("git-like")

    if os.path.exists("naive"):
        shutil.rmtree("naive")

    os.mkdir("git-like")
    os.mkdir("naive")

    step = 0
    step_times = [0 for _ in range(999)]
    repo_size = 200_000
    d1_pairs = {}
    d2_pairs = {}

    for i in range(3):
        new_step(f"init git-like-base with {repo_size} files")

        for _ in range(repo_size):
            file, content = generate_file_and_content()
            parent = os.path.join("git-like", file[:2])

            if len(d1_pairs) < 2000:
                d1_pairs[file] = content

            if not os.path.exists(parent):
                os.mkdir(parent)

            file = os.path.join(parent, file[2:])

            with open(file, "w") as f:
                f.write(content)

        new_step(f"init naive-base with {repo_size} files")

        for _ in range(repo_size):
            file, content = generate_file_and_content()

            if len(d2_pairs) < 2000:
                d2_pairs[file] = content

            file = os.path.join("naive", file)

            with open(file, "w") as f:
                f.write(content)

        new_step("search git-like")

        for key, value in d1_pairs.items():
            file = os.path.join("git-like", key[:2], key[2:])

            with open(file, "r") as f:
                assert int(f.read().strip()) == int(value)

        new_step("search naive")

        for key, value in d2_pairs.items():
            file = os.path.join("naive", key)

            with open(file, "r") as f:
                assert int(f.read().strip()) == int(value)

        if i != 2:
            new_step("rm -r git-like")
            shutil.rmtree("git-like")
            os.mkdir("git-like")
            d1_pairs = {}

            new_step("rm -r naive")
            shutil.rmtree("naive")
            os.mkdir("naive")
            d2_pairs = {}

    new_step("complete!")

    result = {
        "repo_size": repo_size,
        "steps": steps,
    }
    print(json.dumps(result, indent=4))

# run 1: MacOS, M3-Pro, APFS
r = {
    "repo_size": 200000,
    "steps": [
        {
            "seq": 0,
            "title": "init git-like-base with 200000 files",
            "started_at": 1748338102.189761,
            "ended_at": 1748338132.6686232,
            "elapsed_ms": 30478
        }, {
            "seq": 1,
            "title": "init naive-base with 200000 files",
            "started_at": 1748338132.668639,
            "ended_at": 1748338160.474258,
            "elapsed_ms": 27805
        }, {
            "seq": 2,
            "title": "search git-like",
            "started_at": 1748338160.474273,
            "ended_at": 1748338160.596063,
            "elapsed_ms": 121
        }, {
            "seq": 3,
            "title": "search naive",
            "started_at": 1748338160.596082,
            "ended_at": 1748338160.6898549,
            "elapsed_ms": 93
        }, {
            "seq": 4,
            "title": "rm -r git-like",
            "started_at": 1748338160.6898718,
            "ended_at": 1748338190.342999,
            "elapsed_ms": 29653
        }, {
            "seq": 5,
            "title": "rm -r naive",
            "started_at": 1748338190.343023,
            "ended_at": 1748338220.748424,
            "elapsed_ms": 30405
        }, {
            "seq": 6,
            "title": "init git-like-base with 200000 files",
            "started_at": 1748338220.74845,
            "ended_at": 1748338254.644025,
            "elapsed_ms": 33895
        }, {
            "seq": 7,
            "title": "init naive-base with 200000 files",
            "started_at": 1748338254.644044,
            "ended_at": 1748338283.4599879,
            "elapsed_ms": 28815
        }, {
            "seq": 8,
            "title": "search git-like",
            "started_at": 1748338283.4600031,
            "ended_at": 1748338283.5988421,
            "elapsed_ms": 138
        }, {
            "seq": 9,
            "title": "search naive",
            "started_at": 1748338283.59886,
            "ended_at": 1748338283.697797,
            "elapsed_ms": 98
        }, {
            "seq": 10,
            "title": "rm -r git-like",
            "started_at": 1748338283.697817,
            "ended_at": 1748338318.950847,
            "elapsed_ms": 35253
        }, {
            "seq": 11,
            "title": "rm -r naive",
            "started_at": 1748338318.950868,
            "ended_at": 1748338348.511057,
            "elapsed_ms": 29560
        }, {
            "seq": 12,
            "title": "init git-like-base with 200000 files",
            "started_at": 1748338348.5110872,
            "ended_at": 1748338382.013751,
            "elapsed_ms": 33502
        }, {
            "seq": 13,
            "title": "init naive-base with 200000 files",
            "started_at": 1748338382.013765,
            "ended_at": 1748338411.272249,
            "elapsed_ms": 29258
        }, {
            "seq": 14,
            "title": "search git-like",
            "started_at": 1748338411.272267,
            "ended_at": 1748338411.317322,
            "elapsed_ms": 45
        }, {
            "seq": 15,
            "title": "search naive",
            "started_at": 1748338411.317347,
            "ended_at": 1748338411.4253461,
            "elapsed_ms": 107
        }, {
            "seq": 16,
            "title": "complete!",
            "started_at": 1748338411.425366
        }
    ]
}

# run 2: Ubuntu, i7-13700F, ext4
r = {
    "repo_size": 200000,
    "steps": [
        {
            "seq": 0,
            "title": "init git-like-base with 200000 files",
            "started_at": 1748338686.9525535,
            "ended_at": 1748338692.7156608,
            "elapsed_ms": 5763
        },
        {
            "seq": 1,
            "title": "init naive-base with 200000 files",
            "started_at": 1748338692.7156796,
            "ended_at": 1748338698.0063965,
            "elapsed_ms": 5290
        },
        {
            "seq": 2,
            "title": "search git-like",
            "started_at": 1748338698.0064123,
            "ended_at": 1748338698.0172935,
            "elapsed_ms": 10
        },
        {
            "seq": 3,
            "title": "search naive",
            "started_at": 1748338698.0173044,
            "ended_at": 1748338698.0274107,
            "elapsed_ms": 10
        },
        {
            "seq": 4,
            "title": "rm -r git-like",
            "started_at": 1748338698.027416,
            "ended_at": 1748338699.3780425,
            "elapsed_ms": 1350
        },
        {
            "seq": 5,
            "title": "rm -r naive",
            "started_at": 1748338699.3780665,
            "ended_at": 1748338700.4909148,
            "elapsed_ms": 1112
        },
        {
            "seq": 6,
            "title": "init git-like-base with 200000 files",
            "started_at": 1748338700.4909394,
            "ended_at": 1748338705.9839892,
            "elapsed_ms": 5493
        },
        {
            "seq": 7,
            "title": "init naive-base with 200000 files",
            "started_at": 1748338705.9840052,
            "ended_at": 1748338711.1196747,
            "elapsed_ms": 5135
        },
        {
            "seq": 8,
            "title": "search git-like",
            "started_at": 1748338711.1196904,
            "ended_at": 1748338711.130551,
            "elapsed_ms": 10
        },
        {
            "seq": 9,
            "title": "search naive",
            "started_at": 1748338711.1305625,
            "ended_at": 1748338711.1407018,
            "elapsed_ms": 10
        },
        {
            "seq": 10,
            "title": "rm -r git-like",
            "started_at": 1748338711.140707,
            "ended_at": 1748338712.3045957,
            "elapsed_ms": 1163
        },
        {
            "seq": 11,
            "title": "rm -r naive",
            "started_at": 1748338712.30462,
            "ended_at": 1748338713.4315445,
            "elapsed_ms": 1126
        },
        {
            "seq": 12,
            "title": "init git-like-base with 200000 files",
            "started_at": 1748338713.4315727,
            "ended_at": 1748338718.9001195,
            "elapsed_ms": 5468
        },
        {
            "seq": 13,
            "title": "init naive-base with 200000 files",
            "started_at": 1748338718.9001346,
            "ended_at": 1748338724.0638602,
            "elapsed_ms": 5163
        },
        {
            "seq": 14,
            "title": "search git-like",
            "started_at": 1748338724.0638769,
            "ended_at": 1748338724.0747116,
            "elapsed_ms": 10
        },
        {
            "seq": 15,
            "title": "search naive",
            "started_at": 1748338724.0747228,
            "ended_at": 1748338724.084739,
            "elapsed_ms": 10
        },
        {
            "seq": 16,
            "title": "complete!",
            "started_at": 1748338724.0847442
        }
    ]
}

# Conclusion: On all platforms, naive way is better than the git-like way.
# But the difference is very small and I'm not gonna change how ragit stores objects.
