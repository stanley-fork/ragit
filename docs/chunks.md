# Chunks

A chunk is a basic building block of a knowledge-base. Ragit splits data files into chunks, and adds title and summary to each chunk. When it retrieves information, it scores, reranks, and retrieves chunks.

## Data Files

You build a knowledge-base from raw data files, like `.md`, `.txt`, `.pdf`... There are 3 kinds of files: unstaged files, staged files and processed files. If you're familiar with git, you must be familiar with the word "stage". Yes that's it.

When you first run `rag init`, an empty knowledge-base is created. There's no staged file and no processed file. You then have to run `rag add` to stage files. For example, `rag add *.txt` will add all the text files in the directory to the staging area. `rag add` doesn't care whether the file actually exists or not. `rag add file_that_does_not_exist` would work perfectly and just add the file to the staging area.

When you run `rag build`, it pops a file from the staging area, creates chunks for the file, then mark the file as "processed". So, "processed files" are the files who have chunks and ready to be queried. `rag build` does care whether the file actually exists. If you run `rag add file_that_does_not_exist; rag build`, `rag add` would succeed and `rag build` would fail.

Once a file is processed, `rag query` can use the chunks from the file. `rag query` doesn't care whether the original file exists or not. Chunks have all the information that `rag query` needs, and it doesn't try to look for the original file. That means you can delete the original files after `rag build` is complete.

There's another special stage. You can see the term `curr_processing_file` in the source code. Once `rag build` pops a file from the staging area, the file is marked as "curr_processing_file". "curr_processing_file" doesn't belong to "processed_files" and "staged files" until it's fully processed. It matters when you interrupt `rag build`. When you interrupt it, all the chunks of the "curr_processing_file" are discarded, and the "curr_processing_file" goes back to the staging area. Chunks of the "processed_files" will be kept safely.

## Data format

Chunks are saved in a content-addressable way. It's like git's object files. You can find the chunk files in `.ragit/chunks/`, a file per chunk. The first 2 characters of a chunk's uid is the directory name of the chunk file, and the remaining characters in uid consist its file name. For example, if its uid is `abcdef0123`, you'll find the chunk file at `.ragit/chunks/ab/cdef0123.chunk`.
