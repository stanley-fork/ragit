import json
import os
import shutil
import subprocess
from utils import (
    cargo_run,
    count_files,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

reproductions = {
    # found while processing rustc-dev-guide
    "images_in_codefence": "a\n" * 14 + """
```
vec![1]
```

It has something to do with images and code fences.

```
vec![2]
```

Ragit parses markdown files, find images in the document, and replace them with actual images.
But there are a lot of quirks...

```
vec![3]
```

In markdown, an image is a square bracket following "!", and optionally followed by a parenthesis.
In markdown, there's something called "code-fence". You can put (almost) anything inside a code-fence.
Markdown will not parse stuffs inside a code-fence.

```
vec![4]
```

That's where the problem comes. Rust's vector syntax resembles markdown's image syntax.
If a rust vector is inside a code-fence, ragit should not treat it as an image.
Ragit already does that. It parses a markdown file and does not try to find images inside
a code-fence.

```
vec![5]
```

But there was a problem. It's just a bug in the implementation.
Ragit uses a flag which tells whether the cursor is inside a code-fence or not.
If it's inside a code-fence and an ending fence is found, it toggles the flag.
If it's not inside a code-fence and an opening fence is found, it toggles the flag.
But there was a very small edge case where it skipped the check.

```
vec![6]
```

I didn't mean to skip the check. It's just a bug.
It was hard to find because the case is very rare.
It happened only when
  1. there're more than 16 lines in a buffer (then the buffer is flushed)
  2. the cursor is pointing at a code fence
In order to reproduce the bug, I have to place a code-fence at exact location.

```
vec![7]
```

It's not a good idea to place just one code-fence
at the exact location. I might mistakenly edit the content,
or a small change in the file reader implementation might
hide the bug. So I've chosen much more naive way. I'm placing a
lot of code-fences with images, and putting different
gaps between the code-fences, so that it's more likely
to hit the bug.

```
vec![8]
```

I also added a small trick.
Each code block has different content,
so that I can easily know which code-block
is hit by the bug, just by reading the
error message.
Oh no, this paragraph is supposed to be
8-lines long... I'm running out of contents.
haha

```
vec![9]
```

""",

    # found while processing docker
    "absolute_paths": """
This is an image: ![](/empty.png)
This is also an image: ![](/assets/not-empty.webp)
""",

    # found while processing kubernetes
    "images_without_extension": """
This is an image, but it doens't have an extension: ![](http://127.0.0.1:12345/img/sample_image)
""",

    # thought some bugs in kubernetes were due to this,
    # but I later found out that it's not.
    # anyway, I'll keep this test case
    "nested_images": """
[![an image](/empty.png)](link-to-somewhere)
""",

    # found while processing kubernetes
    "url_queries": """
Image 1: ![](http://127.0.0.1:12345/img/sample_image.png?empty=1)
Image 2: ![](http://127.0.0.1:12345/img/sample_image.png?empty=0)

Please check if it has 2 different images.
""",

    # I just came up with this one
    "cannot_fetch_web_images": """
Image 1: ![](http://127.0.0.1:12345/hung)

Please check if it has 0 images.
""",
}

# I ran `python3 tests.py real_repos` and was surprised to see
# it throwing so many errors. Many of them were ragit's fault. So
# I created this test, which tries to reproduce all the errors found
# in the `real_repos` test.
def real_repos_regression():
    goto_root()
    server_process = None

    try:
        mk_and_cd_tmp_dir()
        server_process = subprocess.Popen(["python3", "../tests/simple_image_server.py"])
        shutil.copyfile("../tests/images/empty.png", "empty.png")
        os.mkdir("assets")
        shutil.copyfile("../tests/images/hello_world.webp", "assets/not-empty.webp")

        cargo_run(["init"])
        cargo_run(["config", "--set", "model", "dummy"])
        cargo_run(["config", "--set", "strict_file_reader", "true"])

        for name, content in reproductions.items():
            file_name = name + ".md"
            write_string(file_name, content)
            cargo_run(["add", file_name])

        cargo_run(["build"])

        # `cannot_fetch_web_images` would fail. I want to see if timeout works.
        assert count_files() == (len(reproductions), 1, len(reproductions) - 1)  # (total, staged, processed)

        # step 2: run `rag build` in another directory: I want to make sure that absolute paths work in all directories
        os.mkdir("d")
        os.chdir("d")

        for name, content in reproductions.items():
            file_name = name + ".md"
            write_string(file_name, content)
            cargo_run(["add", file_name])

        cargo_run(["build"])
        assert count_files() == (len(reproductions) * 2, 2, len(reproductions) * 2 - 2)  # (total, staged, processed)

        # check if query string works
        os.chdir("..")
        url_queries = json.loads(cargo_run(["ls-images", "url_queries.md", "--uid-only", "--json"], stdout=True))
        assert len(url_queries) == 2

    finally:
        if server_process is not None:
            server_process.kill()
