# Chunks

A chunk is a basic building block of a knowledge-base. Ragit splits data files into chunks, and adds title and summary to each chunk. When it retrieves information, it scores, reranks, and retrieves chunks.

## Data Files

You have to build a knowledge-base with raw data files, like `.md`, `.txt`, `.pdf`... There are 3 kinds of files: unstaged files, staged files and processed files. If you're familiar with git, you must be familiar with the word "stage". Yes that's it.

When you first run `rag init`, an empty knowledge-base is created. There's no staged file and no processed file. You then have to run `rag add` to stage files. For example, `rag add *.txt` will add all the text files in the directory to the staging area. `rag add` doesn't care whether the file actually exists or not. `rag add file_that_does_not_exist` would work perfectly and just add the file to the staging area.

When you run `rag build`, it pops a file from the staging area, creates chunks for the file, then mark the file as "processed". So, "processed files" are the files who have chunks and ready to be queried. `rag build` does care whether the file actually exists. If you run `rag add file_that_does_not_exist; rag build`, `rag add` would succeed and `rag build` would fail.

Once a file is processed, `rag query` can use the chunks from the file. `rag query` doesn't care whether the original file exists or not. Chunks have all the information that `rag query` needs, and it doesn't try to look for the original file.

There's another special stage. You can see the term `curr_processing_file` in the source code. Once `rag build` pops a file from the staging area, the file is marked as "curr_processing_file". "curr_processing_file" doesn't belong to "processed_files" and "staged files" until it's fully processed. It matters when you interrupt `rag build`. When you interrupt it, all the chunks of the "curr_processing_file" are discarded, and the "curr_processing_file" goes back to the staging area. Chunks of the "processed_files" will be kept safely.

## Chunk Files

You can see the term `chunk_file` in the source code. Chunks have to be stored somewhere in the disk. It's not a good idea to create a file for each chunk. So, multiple chunks are stored in a file, in a json format. We call the files `chunk_file`. You can find chunk files in `.rag_index/chunks` directory. There, you'll find two different files with the same name but different extensions: `.chunks` and `.tfidf`. `.chunks` file stores the actual data, and `.tfidf` stores index for tfidf scores. Both files must have the same chunks, in the same order.

`.tfidf` files are always compressed, and `.chunks` files are compressed depending on its size. You can configure the size threshold and compression level.

TODO: there must be a nicer ui for chunks

## Chunk Index

If you're not a contributor, you don't have to worry what chunk index is.

You can see the term `chunk_index` in the source code. Chunk index is used for optimizations. It makes it easier to find which chunk_file a chunk belongs to using a uid. You'll see the index files in `.rag_index/chunk_index`. There you'll see a scary list of files with short-names. Each file is a `chunk_uid -> chunk_file` map. For example, if a uid of a chunk is "abcdefg", you have to read "ab.json" in the chunk_index directory. In the json file, there must be an entry `"abcdefg": chunk_file_it_belongs_to`.
