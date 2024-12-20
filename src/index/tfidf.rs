use crate::chunk::Chunk;
use crate::error::Error;
use crate::index::Index;
use crate::query::Keywords;
use crate::uid::Uid;
use flate2::Compression;
use flate2::read::{GzDecoder, GzEncoder};
use ragit_api::JsonType;
use ragit_fs::{
    WriteMode,
    read_bytes,
    read_string,
    write_bytes,
};
use rust_stemmers::{Algorithm, Stemmer};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::hash::Hash;
use std::io::Read;

type Term = String;
type Weight = f32;

pub struct TfidfState<DocId> {
    pub terms: HashMap<Term, Weight>,
    term_frequency: HashMap<(DocId, Term), usize>,
    document_frequency: HashMap<Term, usize>,
    document_len: HashMap<DocId, usize>,
    docs: Vec<DocId>,
}

#[derive(Clone)]
pub struct TfidfResult<DocId: Clone> {
    pub id: DocId,
    pub score: f32,
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
pub struct ProcessedDoc {
    pub uid: Option<Uid>,
    pub term_frequency: HashMap<Term, usize>,
    length: usize,
}

// tfidf files are always compressed
pub fn load_from_file(path: &str) -> Result<ProcessedDoc, Error> {
    let content = read_bytes(path)?;
    let mut decompressed = vec![];
    let mut gz = GzDecoder::new(&content[..]);
    gz.read_to_end(&mut decompressed)?;

    Ok(serde_json::from_slice(&decompressed)?)
}

pub fn save_to_file(path: &str, chunk: &Chunk, root_dir: &str) -> Result<(), Error> {
    let tfidf = if chunk.searchable {
        ProcessedDoc::new(chunk.uid.clone(), &chunk.into_tfidf_haystack(root_dir)?)
    } else {
        ProcessedDoc {
            uid: Some(chunk.uid),
            term_frequency: HashMap::new(),
            length: 0,
        }
    };
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

pub fn consume_processed_doc(
    processed_doc: ProcessedDoc,
    tfidf_state: &mut TfidfState<Uid>,
) -> Result<(), Error> {
    tfidf_state.consume(
        processed_doc.uid.unwrap(),
        &processed_doc,
    );
    Ok(())
}

impl ProcessedDoc {
    pub fn new(
        uid: Uid,
        doc_content: &str,
    ) -> Self {
        let mut term_frequency = HashMap::new();
        let mut length = 0;

        for term in tokenize(doc_content) {
            length += 1;

            match term_frequency.get_mut(&term) {
                Some(n) => { *n += 1; },
                None => { term_frequency.insert(term, 1); },
            }
        }

        ProcessedDoc {
            uid: Some(uid),
            length,
            term_frequency,
        }
    }

    pub fn empty() -> Self {
        ProcessedDoc {
            uid: None,
            length: 0,
            term_frequency: HashMap::new(),
        }
    }

    pub fn extend(&mut self, other: &ProcessedDoc) {
        if self.uid != other.uid {
            self.uid = None;
        }

        self.length += other.length;

        for (term, count) in other.term_frequency.iter() {
            match self.term_frequency.get_mut(term) {
                Some(n) => { *n += *count; },
                None => { self.term_frequency.insert(term.clone(), *count); },
            }
        }
    }

    pub fn get(&self, term: &str) -> Option<usize> {
        self.term_frequency.get(term).copied()
    }

    pub fn contains_term(&self, term: &str) -> bool {
        self.term_frequency.contains_key(term)
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn render(&self, term_only: bool, stat_only: bool) -> String {
        let mut lines = vec![];

        if !term_only {
            lines.push(format!(
                "uid: {}, terms: {}, unique_terms: {}",
                if let Some(u) = &self.uid { u.to_string() } else { String::from("None (not from a single chunk)") },
                self.length,
                self.term_frequency.len(),
            ));
        }

        if stat_only {
            return lines[0].clone();
        }

        if !term_only {
            lines.push(String::from("term-frequency:"));
        }

        let mut pairs: Vec<_> = self.term_frequency.iter().collect();
        pairs.sort_by_key(|(_, count)| usize::MAX - *count);

        for (term, count) in pairs.iter() {
            lines.push(format!(
                "{}{term:?}: {count}",
                if term_only { "" } else { "    " },
            ));
        }

        lines.join("\n")
    }
}

impl<DocId: Clone + Eq + Hash> TfidfState<DocId> {
    pub fn new(keywords: &Keywords) -> Self {
        TfidfState {
            terms: keywords.tokenize(),
            term_frequency: HashMap::new(),
            document_frequency: HashMap::new(),
            document_len: HashMap::new(),
            docs: vec![],
        }
    }

    pub fn consume(
        &mut self,
        doc_id: DocId,
        processed_doc: &ProcessedDoc,
    ) {

        for (term, _) in self.terms.clone().iter() {
            if processed_doc.contains_term(term) {
                match self.document_frequency.get_mut(term) {
                    Some(n) => { *n += 1; },
                    None => { self.document_frequency.insert(term.to_string(), 1); },
                }
            }

            self.term_frequency.insert(
                (doc_id.clone(), term.to_string()),
                processed_doc.get(term).unwrap_or(0),
            );
        }

        self.document_len.insert(doc_id.clone(), processed_doc.length());
        self.docs.push(doc_id);
    }

    pub fn get_top(&self, limit: usize) -> Vec<TfidfResult<DocId>> {
        let mut tfidfs: HashMap<DocId, f32> = HashMap::new();

        // https://en.wikipedia.org/wiki/Okapi_BM25
        let k = 1.2;
        let b = 0.75;

        if self.document_len.is_empty() {
            return vec![];
        }

        let avg_len = self.document_len.values().sum::<usize>() as f32 / self.document_len.len() as f32;

        for (term, weight) in self.terms.iter() {
            let idf = ((self.docs.len() + 1) as f32 / (*self.document_frequency.get(term).unwrap_or(&0) + 1) as f32).log2();
            let idf = idf.max(0.1);

            for doc in self.docs.iter() {
                let t = *self.term_frequency.get(&(doc.clone(), term.to_string())).unwrap_or(&0) as f32;

                if t == 0.0 {
                    continue;
                }

                let len = *self.document_len.get(doc).unwrap() as f32;
                let tf = (t * (k + 1.0)) / (t + k * (1.0 - b + b * (len / avg_len)));
                let tfidf = tf * idf;

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

        let mut tfidfs: Vec<_> = tfidfs.into_iter().map(|(id, score)| TfidfResult { id, score }).collect();
        tfidfs.sort_by(|TfidfResult { score: a, .. }, TfidfResult { score: b, .. }| b.partial_cmp(a).unwrap());  // rev sort

        if tfidfs.len() > limit {
            tfidfs[..limit].to_vec()
        } else {
            tfidfs
        }
    }
}

pub fn tokenize(s: &str) -> Vec<String> {
    let stemmer = Stemmer::create(Algorithm::English);
    let mut result = vec![];

    for token in s.to_ascii_lowercase().split(
        |c| if c <= '~' {
            match c {
                '0'..='9'
                | 'A'..='Z'
                | 'a'..='z' => false,
                _ => true,
            }
        } else {
            false
        }
    ).map(
        move |s| ragit_korean::tokenize(&stemmer.stem(s))
    ) {
        for t in token {
            if t.len() > 0 {
                result.push(t);
            }
        }
    }

    result
}

impl Chunk {
    // very naive heuristic
    // 1. `self.title` is very important, so it's included twice
    // 2. `self.file` might have an information.
    // 3. `self.summary` has constraints that are not in `self.data`.
    //     - It has explanations on images
    //     - It's always English
    // 4. Images have to be replaced with its description.
    pub fn into_tfidf_haystack(&self, root_dir: &str) -> Result<String, Error> {
        let mut data = self.data.clone();

        for image in self.images.iter() {
            let description_at = Index::get_image_path(
                root_dir,
                *image,
                "json",
            );
            let j = read_string(&description_at)?;

            let rep_text = match serde_json::from_str::<Value>(&j)? {
                Value::Object(obj) => match (obj.get("extracted_text"), obj.get("explanation")) {
                    (Some(e1), Some(e2)) => format!("<img>{e1}{e2}</img>"),
                    _ => {
                        return Err(Error::BrokenIndex(format!("schema error at {image}.json")));
                    },
                },
                j => {
                    return Err(Error::JsonTypeError {
                        expected: JsonType::Object,
                        got: (&j).into(),
                    });
                },
            };

            data = data.replace(
                &format!("img_{image}"),
                &rep_text,
            );
        }

        Ok(format!(
            "{}\n{}\n{}\n{}\n{}",
            self.render_source(),
            self.title,
            self.title,
            self.summary,
            data,
        ))
    }
}
