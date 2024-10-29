import shutil
from utils import (
    cargo_run,
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

    shutil.copyfile("../tests/images/empty.png", "sample2.png")
    shutil.copyfile("../tests/images/empty.jpg", "sample5.jpg")
    cargo_run(["build"])
    cargo_run(["check"])
