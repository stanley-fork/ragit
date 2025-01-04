# Chunks

A chunk is a basic building block of a knowledge-base. Ragit splits data files into chunks, and adds title and summary to each chunk. When it retrieves information, it scores, reranks, and retrieves chunks.

## Data Files

You build a knowledge-base from raw data files, like `.md`, `.txt`, `.pdf`... There are 3 kinds of files: unstaged files, staged files and processed files. If you're familiar with git, you must be familiar with the word "stage". Yes that's it.

Ragit tries to be as git-like as possible. Staged files are like those in git and processed files are like committed files in git. You run `rag add *.txt` or `rag add --all` to stage files. `rag add` respects `.ragignore` file in the root directory (where `.ragit/` exists). `rag build` pops files from the staging area, create chunks for the file, then mark the file as "processed".

If the file you're trying to `add` is already processed, its behavior depends on whether the file is modified since the last `rag build`. If it has been modified, it's staged again. The previously created chunks are not removed until `rag build` is run. If it hasn't been modified, it's not staged. It's not staged even when `--force` is set. In order to stage such file, you first have to run `rag remove <FILE>`.

Once a file is processed, `rag query` can use the chunks from the file. `rag query` doesn't care whether the original file exists or not. Chunks have all the information that `rag query` needs, and it doesn't try to look for the original file. That means you can delete the original files after `rag build` is complete.

There's another special stage. You can see the term `curr_processing_file` in the source code. Once `rag build` pops a file from the staging area, the file is marked as "curr_processing_file". "curr_processing_file" doesn't belong to "processed_files" and "staged files" until it's fully processed. It matters when you interrupt `rag build`. When you interrupt it, all the chunks of the "curr_processing_file" are discarded, and the "curr_processing_file" goes back to the staging area. Chunks of the "processed_files" will be kept safely.

## Data format

Chunks are saved in a content-addressable way. It's like git's object files. You can find the chunk files in `.ragit/chunks/`, a file per chunk. The first 2 characters of a chunk's uid is the directory name of the chunk file, and the remaining characters in uid consist its file name. For example, if its uid is `abcdef0123`, you'll find the chunk file at `.ragit/chunks/ab/cdef0123.chunk`.
