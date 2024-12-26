# Uid-Query

Uid is like hash of git. For example, `git log` shows you commit hashes, and you use the hashes to branch, checkout, etc. There are also many commands that show you hash values of objects. In ragit, every chunk, file and image have uids. It's a 64 characters hexadecimal string. It's based on sha3-256 hash, but has more metadata.

There are many commands that ask uid from you: `cat-file`, `ls-chunks`, `ls-files`, `ls-images` and `ls-terms`. For example, let's say "abcd1234" is a uid of a file, and "efgh5678" is a uid of a chunk. `rag ls-chunks abcd1234` shows you the chunks that belongs to "abcd1234" and `rag ls-terms efgh5678` shows you the term-frequency of "efab5678".

## Full match

The simplest uid query is to use all the 64 characters. It's faster than prefix-matching.

## Prefix match

Like git, you can use a prefix of a uid. If uid is "1a1cabc9ba4203a0d6c1c862939870da374539db5b8586490000000300003d8c", typing "1a1cabc9" would be enough. Then it would search for all the uids that start with "1a1cabc9", which would be unique. If there are multiple matches, its behavior depends on the implementation of the command. For example, `ls-terms` rejects your request if there are multiple matches, but `ls-chunks` doesn't.

## Path match

It also allows path matches. For example, if there's `docs/index.txt`, `rag cat-file docs/index.txt` would show you the content of the file. It doesn't support prefix-match.
