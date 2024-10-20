## add

See also [chunks.md](./chunks.md#data-files).

- `rag add [FILES..]`
  - ex: `rag add *.txt`
  - ex: `rag add --auto *.txt`

It adds files to the staging area. `rag add` command doesn't check whether `FILE` actually exists or not. If you add an invalid file, `rag build` will raise an error.

`rag add` has 3 options: `--force`, `--auto` and `--ignore`. Its behavior differs depending on whether `FILE` is unstaged, staged or processed.

|                      | `--force`       | `--auto`         | `--ignore`            |
|----------------------|-----------------|------------------|-----------------------|
| `FILE` is unstaged   | Stages `FILE`   | Stages `FILE`    | Stages `FILE`         |
| `FILE` is staged     | Does nothing    | Does nothing     | Does nothing          |
| `FILE` is processed  | Remove chunks and stages `FILE` |  If `FILE` has been updated, removes chunks and stages `FILE`. Otherwise, does nothing  | Does nothing  |

There's an extra flag: `--git`. You can run this command if the knowledge-base you're working on is also a git repository. `rag add --git` adds all the file in the git's working tree to the ragit's staging area.

TODO: `rag add --git` is not implemented yet. It seems like the result of `git ls-tree -r HEAD --name-only` differs on your cwd. How do I manage that? Using `git2` crate?

## build

See also [chunks.md](./chunks.md#data-files).

- `rag build`
- `rag build --dashboard`
  - TODO: Not Implemented Yet

It reads files in the staging area and process them. Once it's processed, you can ask queries on them.

You can interrupt a building at anytime. When interrupted, processed files so far are kept safely, but "curr_processing_file" and its chunks will be discarded. More on [chunks.md](./chunks.md#data-files).

## check

- `rag check`
- `rag check --recursive`
  - Runs check on external knowledge-bases, recursively. (default: false)

It checks if something's broken or corrupted. Normal users don't need this command. If you've added a new feature or are writing a test script, please run this command to see if your codes are correct.

## config

- `rag config --set [key] [value]`
  - ex: `rag config --set model gpt-4o`
- `rag config --get [key]`
  - ex: `rag config --get model`
- `rag config --get-all`

## gc

- `rag gc --logs`
  - It empties `.rag_index/logs` directory.

## init

- `rag init`

It does nothing if there's already a knowledge-base. If you want to re-init, run `rag reset --hard; rag init`.

## ls

- `rag ls --chunks`
  - It does not show external chunks.
- `rag ls --files`
  - It does not show external files.
- `rag ls --models`

## merge

- `rag merge PATH/TO/ANOTHER/KNOWLEDGE/BASE`

It looks for `PATH/TO/ANOTHER/KNOWLEDGE/BASE/.rag_index/index.json`. When another knowledge-base is merged, `rag query` and `rag tfidf` will also search the knowledge-base.

TODO: since `index.json` only saves the relative paths of the merged knowledge-base, it's very tricky to share. There must be more metadata to make it shareable. For example, each external knowledge-base has a url and a relative path, so `rag pull` can pull external ones recursively.

## meta

- `rag meta --get [key]`
  - It doesn't print anything if there's no such key.
- `rag meta --get-all`
- `rag meta --set [key] [value]`
- `rag meta --remove [key]`
  - It does nothing if there's no such key.
- `rag meta --remove-all`

You can add metadata to a knowledge-base. Metadata is a json object, where all the keys and values are strings. You can find the actual file at `.rag_index/meta.json`.

TODO: I want values to have various types, but that's too trick to implement. For example, in most shells, `rag meta --set num 1` and `rag meta --set num "1"` are both integer and `rag meta --set num "\"1\""` is a string.

## pull

- `rag pull URL/TO/ANOTHER/KNOWLEDGE/BASE`
  - TODO: not implemented yet

You can download knowledge-bases from the internet. It's not available yet, so I provide you links that you can download knowledge-bases created by me.

- [docker](TODO)
- [git](TODO)
- [kubernetes](TODO)
- [nix](TODO)
- [postgresql](TODO)
- [ragit](TODO)
- [rustc-dev-guide](TODO)

## query

- `rag query [query]`
  - ex: `rag query "why is the sky blue?"`
- `rag query --interactive`

## remove

See also [chunks.md](./chunks.md#data-files).

- `rag remove [FILES..]`
  - If `FILE` is staged or processed, the file is removed in the knowledge-base and becomes unstaged.
  - It's like `git restore --unstaged` and `git rm --cached`. It doesn't remove the actual file.
- `rag remove --auto`
  - It iterates all the files in the "staged_files" and "processed_files", and checks if the files actually exists. It removes the files that do not exist anymore.

An example usage of `rag remove --auto` would be:

1. You have tons of text files, and you have built a knowledge-base on that.
2. The text files are updated, and some of them are removed.
3. Now your knowledge-base is outdated. You have to remove the chunks of the removed files.
4. Run `rag remove --auto` to remove them.

## reset

- `rag reset --hard`
  - It resets everything. It removes `.rag_index` dir. You can run `rag init` again to start from scratch.
- `rag reset --soft`
  - It removes all the chunks, staged files and processed files, you have to run `rag add` and `rag build` again from scratch.
  - Configs, logs and prompts are still kept.

## sync

- `rag sync --git`
  - `rag remove --auto` + `rag add --auto --git`
  - TODO: not implemented yet

## tfidf

- `rag tfidf [KEYWORDS..]`
  - keyword-search on your knowledge-base
- `rag tfidf --show [UID]`
  - TODO: uid of files (it only supports chunks)
