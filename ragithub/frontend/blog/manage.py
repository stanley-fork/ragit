from hashlib import sha3_256
import json
import os
import re
import sys

help_message = """
Whatever you're doing, please make sure that you're at `ragithub/blog/`.

# Article metadata

Each article shall begin with [a metadata block](https://baehyunsol.github.io/MDxt-Reference.html#metadata).

The server, which uses [MDxt](https://github.com/baehyunsol/MDxt), can parse yaml, but this python script can only parse json.
So you must use json syntax, which is a subset of yaml.

# How to add an article

1. On whatever machine
  - Write the article.
  - Git add, commit and push.
2. On the server
  - Git pull.
  - Run `python3 manage.py create_index`.
"""

if __name__ == "__main__":
    command = None if len(sys.argv) < 2 else sys.argv[1]

    if command == "create_index":
        index = []

        for article in os.listdir():
            if not article.endswith(".md"):
                continue

            with open(article, "r") as f:
                md = f.read()

            metadata = re.match(r"^---\n(.+)\n---", md, flags=re.DOTALL).group(1)
            metadata = json.loads(metadata)

            if not re.match(r"^\d{4}-\d{2}-\d{2}$", metadata["date"]):
                raise ValueError(f'schema error at field ["date"] of `{article}`: {metadata["date"]}')

            index.append({
                # It uses a hash of the file name to identify articles because
                #
                # 1. It could be a vulnerability to use file names directly.
                # 2. Hash values are shorter.
                # 3. *I love using hexadecimal values.* That's why you can see tons of hash values in `../test-results/`.
                "key": sha3_256(article.encode("utf-8")).hexdigest()[:9],

                "title": metadata["title"],
                "date": metadata["date"],
                "author": metadata["author"],
                "tags": metadata.get("tags", []),
                "file": article,
            })

        index.sort(key=lambda a: a["date"])
        index = index[::-1]

        with open("_index.json", "w") as f:
            f.write(json.dumps(index, ensure_ascii=False, indent=4))

    else:
        print(help_message)
