# Benchmarks

TODO

I really want to have an automated benchmarks.

## What I want

1. Prepare a set of questions and answers. I prefer the set in plain-text file, like json, but any format would do. The questions must be machine-evaluable, so that the entire pipeline can be automated.
2. Run the benchmark with different models and configs. The result is also saved as a file.

## What I want from it

1. *More robust testing*. Let's say you have changed how chunking works. If a benchmark fails or the score is very low, it's likely that your new code has a bug.
2. Comparison between different models and configs.
