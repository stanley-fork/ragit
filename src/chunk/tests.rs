use crate::chunk::{Chunk, ChunkSource, RenderableChunk};
use crate::index::Index;
use super::merge_and_convert_chunks;

#[test]
fn test_merge_and_convert_chunks() {
    let samples = vec![
        (vec![], vec![]),
        (vec![("abc", 0, 0)], vec![("abc", 0, 0)]),

        // no merge
        (vec![("abc", 0, 3), ("가나다", 2, 1), ("123", 5, 5)], vec![("abc", 0, 3), ("가나다", 2, 1), ("123", 5, 5)]),
        (vec![("abc", 0, 0), ("가나다", 2, 0), ("123", 5, 0)], vec![("abc", 0, 0), ("가나다", 2, 0), ("123", 5, 0)]),
        (vec![("abc", 0, 2), ("가나다", 2, 2), ("123", 5, 2)], vec![("abc", 0, 2), ("가나다", 2, 2), ("123", 5, 2)]),

        // merge chunks
        (vec![("abc", 0, 0), ("def", 0, 1)], vec![("abcdef", 0, 0)]),
        (vec![("def", 0, 1), ("abc", 0, 0)], vec![("abcdef", 0, 0)]),

        // merge a lot of chunks
        (vec![("abc", 0, 1), ("def", 0, 2), ("ghi", 0, 3)], vec![("abcdefghi", 0, 1)]),
        (vec![("abc", 0, 1), ("def", 0, 2), ("ghi", 0, 3), ("가나다", 1, 4)], vec![("abcdefghi", 0, 1), ("가나다", 1, 4)]),
        (vec![("abc", 0, 1), ("def", 0, 2), ("ghi", 0, 3), ("jkl", 0, 4)], vec![("abcdefghijkl", 0, 1)]),

        // If the LLM accidentally chose the same chunk twice
        // Or there's a bug (https://github.com/baehyunsol/ragit/issues/8)
        (vec![("abc", 0, 0), ("def", 0, 0)], vec![("def", 0, 0)]),

        // The result has to be sorted by `index`.
        (vec![("abc", 0, 0), ("ghi", 0, 2)], vec![("abc", 0, 0), ("ghi", 0, 2)]),
        (vec![("ghi", 0, 2), ("abc", 0, 0)], vec![("abc", 0, 0), ("ghi", 0, 2)]),
        (vec![("abc", 0, 0), ("ghi", 0, 2), ("가나다", 1, 3)], vec![("abc", 0, 0), ("ghi", 0, 2), ("가나다", 1, 3)]),
        (vec![("ghi", 0, 2), ("abc", 0, 0), ("가나다", 1, 3)], vec![("abc", 0, 0), ("ghi", 0, 2), ("가나다", 1, 3)]),
    ];
    let samples = samples.into_iter().map(
        |(sample, answer)| (
            sample.into_iter().map(
                |(content, file, index)| Chunk::dummy(content.to_string(), ChunkSource::File { path: file.to_string(), index })
            ).collect::<Vec<_>>(),
            answer,
        )
    ).collect::<Vec<_>>();
    let index = Index::dummy();

    for (sample, answer) in samples.into_iter() {
        let result = merge_and_convert_chunks(&index, sample, true).unwrap();
        let answer = answer.into_iter().map(
            |(data, file, index)| RenderableChunk { data: data.to_string(), source: ChunkSource::File { path: file.to_string(), index }.render() }
        ).collect::<Vec<_>>();

        assert_eq!(result, answer);
    }
}
