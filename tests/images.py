import shutil
from utils import (
    cargo_run,
    count_images,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

sample_markdown = '''
# Title

```code
![image](sample1.png) -> this is not an image
```

![image](sample2.png) -> this is an image

`![image](sample3.png) -> this is not an image`, `![image](sample4.png)` -> this is not an image

![sample5] -> this is an image

[sample5]: sample5.jpg

![this is an image][sample6]

[sample6]: sample6.webp

Let's see if ragit's chunk engine can handle sequences of images.

![sample6]![sample6]![sample6]![sample6]

![image](sample2.png)![image](sample2.png)![image](sample2.png)![image](sample2.png)

'''

def images():
    goto_root()
    mk_and_cd_tmp_dir()
    write_string("sample.md", sample_markdown)

    # step 0: initialize knowledge-base
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["config", "--set", "strict_file_reader", "true"])
    cargo_run(["add", "sample.md"])
    cargo_run(["check"])

    stderr = cargo_run(["build"], stderr=True, check=False)
    assert "sample2.png" in stderr   # "sample2.png not found" is expected
    cargo_run(["check", "--recover"])

    shutil.copyfile("../tests/images/empty.png", "sample2.png")
    shutil.copyfile("../tests/images/empty.jpg", "sample5.jpg")
    shutil.copyfile("../tests/images/empty.webp", "sample6.webp")
    cargo_run(["build"])
    cargo_run(["check"])

    # step 1: `rm` does not remove images, but `gc-images` does
    assert count_images() == 3
    cargo_run(["rm", "sample.md"])
    assert count_images() == 3
    assert "removed 6 files" in cargo_run(["gc", "--images"], stdout=True)  # 3 images and 3 descriptions
    assert count_images() == 0
