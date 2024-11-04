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

pdf to markdown

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

futher indexing: `token -> chunk_file` map for the entire knowledge-base

1. try https://github.com/meilisearch/heed
2. my own database

---

fake chunks

ex: "give me an overview of this project" for a source code repo, or auto-generated source map (no need for llms)

do I need `.rag_index/fake_data`?

---

providing the summary of the entire knowledge-base when extracting keywords?

it would be better to provide the summary when generating a summary... but how? in order to bootstrap, it has to generate summary twice. is that worth it?

---

we need some kinda lock for the read-only flag in index

---

batch api for summarizing

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

unique identifiers

1. chunk
  - uses hash of tfidf_haystack: good
2. chunk file
  - xor of uids of its chunks: good
3. data file
  - uses (length + hash) of the file: good
  - for the sake of uniqueness, hashing is enough. but the length adds an extra readability
4. index
  - no identifiers: do we need one?

---

it uses both `json` and `serde_json`, which makes the code so confusing.

---

internal merging:

run `rag build` on multiple machines on the same data, and merge them into a single knowledge-base
