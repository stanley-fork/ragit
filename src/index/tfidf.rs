use crate::chunk::{Chunk, Uid};
use crate::error::Error;
use crate::query::Keywords;
use flate2::Compression;
use flate2::read::{GzDecoder, GzEncoder};
use ragit_fs::{
    WriteMode,
    read_bytes,
    write_bytes,
};
use rust_stemmers::{Algorithm, Stemmer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use std::io::Read;

type Path = String;
type Keyword = String;
type Weight = f32;

pub struct TfIdfState<DocId> {
    keywords: HashMap<Keyword, Weight>,
    tf: HashMap<(DocId, Keyword), f32>,
    doc_count: usize,
    keyword_in_doc: HashMap<Keyword, usize>,
    docs: Vec<DocId>,
}

#[derive(Clone)]
pub struct TfIdfResult<DocId: Clone> {
    pub id: DocId,
    pub score: f32,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ProcessedDoc {
    pub chunk_uid: Option<Uid>,
    pub tokens: HashMap<String, usize>,
    length: usize,
}

// tfidf files are always compressed
pub fn load_from_file(path: &str) -> Result<Vec<ProcessedDoc>, Error> {
    let content = read_bytes(path)?;
    let mut decompressed = vec![];
    let mut gz = GzDecoder::new(&content[..]);
    gz.read_to_end(&mut decompressed)?;

    Ok(serde_json::from_slice(&decompressed)?)
}

pub fn save_to_file(path: &str, chunks: &[Chunk]) -> Result<(), Error> {
    let tfidf = chunks.iter().map(
        |chunk| ProcessedDoc::new(chunk.uid.clone(), &chunk.into_tfidf_haystack())
    ).collect::<Vec<_>>();
    let result = serde_json::to_vec(&tfidf)?;
    let mut compressed = vec![];
    let mut gz = GzEncoder::new(&result[..], Compression::best());
    gz.read_to_end(&mut compressed)?;

    Ok(write_bytes(
        path,
        &compressed,
        WriteMode::CreateOrTruncate,
    )?)
}

pub fn consume_tfidf_file(
    path: Path,  // real path
    ignored_chunks: &[Uid],
    tfidf_state: &mut TfIdfState<Uid>,
) -> Result<(), Error> {
    let processed_docs = load_from_file(&path)?;

    // processed_docs returned from `load_from_file` must have uids
    for processed_doc in processed_docs.iter() {
        if ignored_chunks.contains(processed_doc.chunk_uid.as_ref().unwrap()) {
            continue;
        }

        tfidf_state.consume(processed_doc.chunk_uid.clone().unwrap(), &processed_doc);
    }

    Ok(())
}

impl ProcessedDoc {
    pub fn new(
        chunk_uid: Uid,
        doc_content: &str,
    ) -> Self {
        let mut tokens = HashMap::new();
        let mut length = 0;

        for token in tokenize(doc_content) {
            length += 1;

            match tokens.get_mut(&token) {
                Some(n) => { *n += 1; },
                None => { tokens.insert(token, 1); },
            }
        }

        ProcessedDoc {
            chunk_uid: Some(chunk_uid),
            length,
            tokens,
        }
    }

    pub fn empty() -> Self {
        ProcessedDoc {
            chunk_uid: None,
            length: 0,
            tokens: HashMap::new(),
        }
    }

    pub fn extend(&mut self, other: &ProcessedDoc) {
        if self.chunk_uid != other.chunk_uid {
            self.chunk_uid = None;
        }

        self.length += other.length;

        for (token, count) in other.tokens.iter() {
            match self.tokens.get_mut(token) {
                Some(n) => { *n += *count; },
                None => { self.tokens.insert(token.clone(), *count); },
            }
        }
    }

    pub fn get(&self, token: &str) -> Option<usize> {
        self.tokens.get(token).copied()
    }

    pub fn contains_key(&self, token: &str) -> bool {
        self.tokens.contains_key(token)
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn render(&self) -> String {
        let mut lines = vec![];
        lines.push(format!("chunk uid: {}", if let Some(u) = &self.chunk_uid { u.to_string() } else { String::from("None (not from a single chunk)") }));
        lines.push(format!("tokens: {}", self.length));
        lines.push(String::from("term-frequency:"));

        let mut pairs: Vec<_> = self.tokens.iter().collect();
        pairs.sort_by_key(|(_, count)| usize::MAX - *count);

        for (token, count) in pairs.iter() {
            lines.push(format!("    {token:?}: {count}"));
        }

        lines.join("\n")
    }
}

impl<DocId: Clone + Eq + Hash> TfIdfState<DocId> {
    pub fn new(keywords: &Keywords) -> Self {
        TfIdfState {
            keywords: keywords.tokenize(),
            tf: HashMap::new(),
            doc_count: 0,
            keyword_in_doc: HashMap::new(),
            docs: vec![],
        }
    }

    pub fn consume(
        &mut self,
        doc_id: DocId,
        processed_doc: &ProcessedDoc,
    ) {
        self.doc_count += 1;

        for (keyword, _) in self.keywords.clone().iter() {
            if processed_doc.contains_key(keyword) {
                match self.keyword_in_doc.get_mut(keyword) {
                    Some(n) => { *n += 1; },
                    None => { self.keyword_in_doc.insert(keyword.to_string(), 1); },
                }
            }

            self.tf.insert(
                (doc_id.clone(), keyword.to_string()),
                processed_doc.get(keyword).unwrap_or(0) as f32 / processed_doc.length() as f32,
            );
        }

        self.docs.push(doc_id);
    }

    pub fn get_top(&self, max_len: usize) -> Vec<TfIdfResult<DocId>> {
        let mut tfidfs: HashMap<DocId, f32> = HashMap::new();

        for (keyword, weight) in self.keywords.iter() {
            let idf = ((self.doc_count as f32 + 1.0) / (*self.keyword_in_doc.get(keyword).unwrap_or(&0) as f32 + 1.0)).log2();

            for doc in self.docs.iter() {
                let tfidf = *self.tf.get(&(doc.clone(), keyword.to_string())).unwrap_or(&0.0) * idf;

                // #[cfg(test)] {
                //     println!("{doc:?}/{keyword}: {}", tfidf * 1000.0);
                // }

                if tfidf == 0.0 {
                    continue;
                }

                match tfidfs.get_mut(doc) {
                    Some(val) => {
                        *val += tfidf * weight;
                    },
                    None => {
                        tfidfs.insert(doc.clone(), tfidf * weight);
                    },
                }
            }
        }

        let mut tfidfs: Vec<_> = tfidfs.into_iter().map(|(id, score)| TfIdfResult { id, score }).collect();
        tfidfs.sort_by(|TfIdfResult { score: a, .. }, TfIdfResult { score: b, .. }| b.partial_cmp(a).unwrap());  // rev sort

        if tfidfs.len() > max_len {
            tfidfs[..max_len].to_vec()
        } else {
            tfidfs
        }
    }
}

pub fn tokenize(s: &str) -> Vec<String> {
    let stemmer = Stemmer::create(Algorithm::English);  // TODO: configurable?
    s.to_ascii_lowercase().split(
        |c| match c {
            '\n' | '\t' | '\r'
            | ' ' | '!' | '"' | '\''
            | '(' | ')' | ',' | '-'
            | '.' | '/' | ':' | ';'
            | '[' | ']' | '_' | '`'
            | '{' | '}' => true,
            _ => false,
        }
    ).map(
        move |s| stemmer.stem(s).to_string()
    ).filter(
        |s| s.len() > 0
    ).collect()
}

impl Chunk {
    // chunk's uid is also built from this haystack
    pub fn into_tfidf_haystack(&self) -> String {
        // very naive heuristic
        // 1. `self.title` is very important, so it's included twice
        // 2. `self.file` might have an information.
        // 3. `self.summary` has constraints that are not in `self.data`.
        //     - It has explanations on images
        //     - It's always English
        // 4. TODO: process images in `self.data` so that images are tfidf-able
        format!(
            "{}\n{}\n{}\n{}\n{}",
            self.file,
            self.title,
            self.title,
            self.summary,
            self.data,
        )
    }
}
