---
{
    "title": "ragit 0.3.1 release (hotfix)",
    "date": "2025-02-26",
    "author": "baehyunsol",
    "tags": ["release"]
}
---

# ragit 0.3.1 release (hotfix)

2025-02-26

It's a hotfix, incorporating feedback received since the launch of version 0.3.0.

## Dependencies

No changes

## hotfix gh issue #8

[issue]

It does not fix the root cause of the [issue], but prevents the ragit from crashing.

In previous versions, `merge_and_convert_chunks` failed if there are multiple chunks that have the same `ChunkSource`. It threw an assertion error. I removed the assertions and added a dedicated test suite for the function.

[issue]: https://github.com/baehyunsol/ragit/issues/8

## fetch images from web

Some markdown files want to fetch images from web, like `![image](https://some.url/image.png)`. Now ragit can handle such images. It first downloads the images to disk, then treats them like other images.

## `rag init`

Previously, `rag init` does not create `.ragit/models.json`. You had to run another command which calls `Index::load` which initializes `.ragit/models.json`. It doesn't make sense that you have to run an arbitrary command to get `models.json`. Now the json file is created at `rag init`.

## tests

2 tests are added: images3 and web_images.
