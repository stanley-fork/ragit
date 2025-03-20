// Tree: it turns a file or a directory to a tree-like structure, then make LLMs to generate a summary of the tree.
//
// Let's say you want a summary of a file. If the file is small enough, you can just give the entire file to the LLM
// and ask if for a summary. If it's not small enough, but still a few chunks long, you can give the summaries of the
// chunks and ask for a summary. But what if the file is 1000 chunks long? You can't give 1000 chunks to the LLM. Maybe
// you could, but it's not scalable. So it first creates a tree like below.
//
//                *---------------- summary ----------------*
//                |                                         |
//               ...                                       ...
//                |                                         |
//     *--- fake chunk 1  ---*      ...      *------- fake chunk M  -------*
//     |          |          |               |              |              |
// (chunk 1)  (chunk 2)  (chunk 3)  ...  (chunk N - 2)  (chunk N - 1)  (chunk N)
//
// `chunk 1` ~ `chunk N` are real chunks. Let's say there are too many chunks for the LLM to handle at once. So ragit
// first gives the first 3 chunks to the LLM and asks it to summary the chunks. It doesn't have to be 3; the number is
// configurable. Then it gives the next 3 chunks, on and on. After the first iteration, it'll have M fake chunks. If
// M is small enough, the LLM can consume the fake chunks and generate a summary of the file. If it's not, it would
// recurse.
//
// NOTE: It has nothing to do with KAG (Knowledge graph Augmented Generation). It doesn't care about relevance between chunks.

mod file;
mod dir;

#[derive(Clone, Debug, PartialEq)]
pub enum Tree {
    Leaf(Leaf),
    Node(Vec<Tree>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Leaf {
    // how many chunks this leaf has
    Count(usize),

    // this leaf points to this range of chunks
    // `from` is inclusive and `to` is exclusive
    Range { from: usize, to: usize },
}

impl Tree {
    #[cfg(test)]
    pub fn count_chunks(&self) -> usize {
        match self {
            Tree::Leaf(Leaf::Count(n)) => *n,
            Tree::Leaf(Leaf::Range { from, to }) => *to - *from,
            Tree::Node(t) => t.iter().map(|t| t.count_chunks()).sum(),
        }
    }

    // It converts `Leaf::Count` to `Leaf::Range`
    pub(crate) fn mark_range(&mut self, cur: &mut usize) {
        match self {
            Tree::Leaf(Leaf::Count(n)) => {
                let n = *n;
                *self = Tree::Leaf(Leaf::Range { from: *cur, to: *cur + n });
                *cur += n;
            },
            Tree::Leaf(Leaf::Range { .. }) => unreachable!(),
            Tree::Node(t) => {
                for child in t.iter_mut() {
                    child.mark_range(cur);
                }
            },
        }
    }

    // Do not call this function after `mark_range`.
    #[allow(unused)]
    pub(crate) fn flatten(&self) -> Vec<usize> {
        match self {
            Tree::Leaf(leaf) => vec![leaf.unwrap_count()],
            Tree::Node(leaves) => {
                let mut result = Vec::with_capacity(leaves.len());

                for leaf in leaves.iter() {
                    match leaf {
                        Tree::Leaf(leaf) => { result.push(leaf.unwrap_count()); },
                        Tree::Node(_) => { result.append(&mut leaf.flatten()); },
                    }
                }

                result
            },
        }
    }

    pub(crate) fn flatten_range(&self) -> Vec<(usize, usize)> {
        match self {
            Tree::Leaf(leaf) => vec![leaf.unwrap_range()],
            Tree::Node(leaves) => {
                let mut result = Vec::with_capacity(leaves.len());

                for leaf in leaves.iter() {
                    match leaf {
                        Tree::Leaf(leaf) => { result.push(leaf.unwrap_range()); },
                        Tree::Node(_) => { result.append(&mut leaf.flatten_range()); },
                    }
                }

                result
            },
        }
    }
}

impl Leaf {
    pub(crate) fn unwrap_count(&self) -> usize {
        match self {
            Leaf::Count(n) => *n,
            _ => panic!(),
        }
    }

    pub(crate) fn unwrap_range(&self) -> (usize, usize) {
        match self {
            Leaf::Range { from, to } => (*from, *to),
            _ => panic!(),
        }
    }
}

pub fn generate_tree(total: usize, n: usize) -> Tree {
    assert!(n > 1);

    if total <= n {
        Tree::Leaf(Leaf::Count(total))
    }

    else {
        let mut curr = n;

        while curr < total {
            curr *= n;
        }

        let max_child_size = curr / n;
        let mut child_count = total / max_child_size;

        if total % max_child_size != 0 {
            child_count += 1;
        }

        let mut each_child_size = vec![total / child_count; child_count];

        for i in 0..(total % child_count) {
            each_child_size[i] += 1;
        }

        Tree::Node(each_child_size.into_iter().map(|c| generate_tree(c, n)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::{Leaf, Tree, generate_tree};

    #[test]
    fn tree_correctness() {
        for i in 1..100 {
            for j in 2..10 {
                let t = generate_tree(i, j);
                assert_eq!(i, t.count_chunks());
            }
        }
    }

    #[test]
    fn tree_common() {
        assert_eq!(generate_tree(8, 10), Tree::Leaf(Leaf::Count(8)));
        assert_eq!(generate_tree(12, 10), Tree::Node(vec![Tree::Leaf(Leaf::Count(6)), Tree::Leaf(Leaf::Count(6))]));
        assert_eq!(generate_tree(30, 10), Tree::Node(vec![Tree::Leaf(Leaf::Count(10)), Tree::Leaf(Leaf::Count(10)), Tree::Leaf(Leaf::Count(10))]));
        assert_eq!(generate_tree(30, 10).flatten(), vec![10, 10, 10]);
        assert_eq!(generate_tree(210, 10), Tree::Node(vec![
            Tree::Node(vec![
                Tree::Leaf(Leaf::Count(10)), Tree::Leaf(Leaf::Count(10)), Tree::Leaf(Leaf::Count(10)), Tree::Leaf(Leaf::Count(10)),
                Tree::Leaf(Leaf::Count(10)), Tree::Leaf(Leaf::Count(10)), Tree::Leaf(Leaf::Count(10)),
            ]),
            Tree::Node(vec![
                Tree::Leaf(Leaf::Count(10)), Tree::Leaf(Leaf::Count(10)), Tree::Leaf(Leaf::Count(10)), Tree::Leaf(Leaf::Count(10)),
                Tree::Leaf(Leaf::Count(10)), Tree::Leaf(Leaf::Count(10)), Tree::Leaf(Leaf::Count(10)),
            ]),
            Tree::Node(vec![
                Tree::Leaf(Leaf::Count(10)), Tree::Leaf(Leaf::Count(10)), Tree::Leaf(Leaf::Count(10)), Tree::Leaf(Leaf::Count(10)),
                Tree::Leaf(Leaf::Count(10)), Tree::Leaf(Leaf::Count(10)), Tree::Leaf(Leaf::Count(10)),
            ]),
        ]));
        assert_eq!(generate_tree(210, 10).flatten(), vec![
            10, 10, 10, 10, 10, 10, 10,
            10, 10, 10, 10, 10, 10, 10,
            10, 10, 10, 10, 10, 10, 10,
        ]);
        assert_eq!(generate_tree(30, 8), Tree::Node(vec![Tree::Leaf(Leaf::Count(8)), Tree::Leaf(Leaf::Count(8)), Tree::Leaf(Leaf::Count(7)), Tree::Leaf(Leaf::Count(7))]));
        assert_eq!(generate_tree(30, 8).flatten(), vec![8, 8, 7, 7]);
    }

    #[test]
    fn tree_to_range() {
        let mut t = generate_tree(30, 5);
        t.mark_range(&mut 0);

        assert_eq!(
            t,
            Tree::Node(vec![
                Tree::Node(vec![
                    Tree::Leaf(Leaf::Range { from: 0, to: 5 }), Tree::Leaf(Leaf::Range { from: 5, to: 10 }), Tree::Leaf(Leaf::Range { from: 10, to: 15 }),
                ]),
                Tree::Node(vec![
                    Tree::Leaf(Leaf::Range { from: 15, to: 20 }), Tree::Leaf(Leaf::Range { from: 20, to: 25 }), Tree::Leaf(Leaf::Range { from: 25, to: 30 }),
                ]),
            ]),
        );
        assert_eq!(
            t.flatten_range(),
            vec![(0, 5), (5, 10), (10, 15), (15, 20), (20, 25), (25, 30)],
        );
    }
}
