# ragit-korean

Ragit-korean is a very simple korean tokenizer.

Ragit used to use [charabia](https://github.com/meilisearch/charabia) to tokenize cjk documents, but it has too many issues.

1. Charabia bundles cjk dictionaries in the binary, which makes the file 70MiB bigger.
2. It silently converts 완성형 korean to 조합형 korean. That silently messes up tfidf searches.
