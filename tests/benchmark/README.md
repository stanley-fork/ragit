# Ragit Benchmarks

TODO

## 1. Rerank Summary

A testset consists of (summaries, query, relevant summaries).

It runs `rerank_summary.pdl` with the set, and counts how many tuples it get correct. The problem is that it's tough to tell whether a summary is relenvant or not. It might seem relevant to someone and not relevant to another.

## 2. End-to-End

A testset consists of a knowledge-base and a set of questions and answers (multi-choice).

It first runs the testset without RAG, then with RAG.

It counts the number of questions that it got correct with RAG, but failed without RAG.
