why not github issues?

---

chunk merging: implemented, but not tested

---

try mediawiki to markdown conversion in pandoc

---

multi-languages

|                | korean document  | english document  |
|----------------|------------------|-------------------|
| korean query   |
| english query  |

---

images

1. markdown
  - build a very simple markdown parser that finds `![]()` patterns (while respecting code fences)
  - replace those patterns with images
  - there are tons of more things to do
2. pdf
  - https://github.com/VikParuchuri/marker
  - https://github.com/facebookresearch/nougat
  - https://github.com/DS4SD/docling

---

more commands

- modify config files
- view config files
- view index file

---

more on config command

- `rag config --api -e`
  - `-e` opens an editor
  - `--api`, `--query`, `--build`
  - global vs local?

---

further indexing? what if there are more than million, or even billion chunks?

1. run vector search to retrieve a few thousand chunks. run tf-idf on the chunks
2. there must be algorithms for making an index for tf-idf. it's a few decades old technology
3. run `ProcessedDoc::merge` on 30 ~ 100 chunks. first run tf-idf on merged chunks, retrieve a few merged chunks, then run tf-idf on real chunks
  - how do I cluster chunks?
  - if there's a good clustering algorithm, we can run tf-idf in O(log n)-ish instead of O(n)

---

fake chunks

ex: "give me an overview of this project" for a source code repo, or auto-generated source map (no need for llms)

do I need `.rag_index/fake_data`?

---

providing the summary of the entire knowledge-base when extracting keywords?

it would be better to provide the summary when generating a summary... but how? in order to bootstrap, it has to generate summary twice. is that worth it?

---

base64 for pdl, like `<|image_raw(png/abcd1234hhhh)|>`

---

we need some kinda lock for the read-only flag in index

---

batch api for summarizing

---

another build configuration: error handlings

file readers may fail. for example,

1. `PlainTextReader` encounters an invalid utf-8
2. syntax errors in csv or markdown, or a corrupted pdf file
3. `![image](path)` in markdown, but cannot find `path`
4. ... and many more

it may

1. panic
2. ignore this file and continue with the next file
3. ignore the error and continue

---

another api configuration: models depending on context

for example,

gpt-4o-mini for summarizing documents with images, llama 70B for without images and sonnet for answering queries.

Or even more fine-grained: set model for each prompt (with/without images)

there must be options for defaults, too

---

more tests on further question

q by human, a by llm -> switch roles

```
llm: q
human: a
llm: thanks! i have a question regarding your answer
human: what is that?
llm:
```

would it be worth trying?

---

prompts... I need multi-step problem solving

when there's a query Q

1. answer queries that would help understand Q
2. retrieve chunks for Q
3. ask further questions on the chunks
4. combine all

---

There are some commands that does not require a knowledge-base, like `rag ls --models`, but fails when a knowledge-base is not found

---

unique identifiers

1. chunk
  - uses hash of (data + title + summary): good
2. chunk file
  - uses a randomly generated name: bad
3. data file
  - uses (length + hash) of the file: good
  - for the sake of uniqueness, hashing is enough. but the length adds an extra readability
4. index
  - uses a randomly generated name: bad

---

it uses both `json` and `serde_json`, which makes the code so confusing.

---

internal merging:

run `rag build` on multiple machines on the same data, and merge them into a single knowledge-base

---

push/pull

1. `chunk_index/`: we can build this locally
2. `chunks/`: has to be sent -> let's send it file by file
  - `.tfidf` files can be and should be built locally
3. `configs/`: has to be sent -> not a big deal
4. `images/`: has to be sent -> let's send it file by file
5. `prompts/`: has to be sent -> not a big deal
6. `index.json`: has to be sent
  - to implement pause/resume-able sends, there must be some kinda state stored in this file
7. `usages.json`: don't need this

server <-> client

1. client asks the list of the files to the server
2. client asks a file in the list to the server, one by one

It makes servers very easy to implement: it only needs 2 api: file_list and file

what if there's a partial update? what if one wants to pull only the updated chunks?
