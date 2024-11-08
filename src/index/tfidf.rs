use charabia::{Language, TokenizerBuilder};
use crate::chunk::{Chunk, Uid};
use crate::error::Error;
use crate::index::Index;
use crate::query::Keywords;
use flate2::Compression;
use flate2::read::{GzDecoder, GzEncoder};
use json::JsonValue;
use ragit_api::{
    JsonType,
    get_type,
};
use ragit_fs::{
    WriteMode,
    read_bytes,
    read_string,
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
    keyword_in_doc: HashMap<Keyword, usize>,
    docs: Vec<DocId>,
}

#[derive(Clone)]
pub struct TfIdfResult<DocId: Clone> {
    pub id: DocId,
    pub score: f32,
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
pub struct ProcessedDoc {
    pub chunk_uid: Option<Uid>,
    pub tokens: HashMap<String, usize>,
    length: usize,
}

// tfidf files are always compressed
pub fn load_from_file(path: &Path) -> Result<Vec<ProcessedDoc>, Error> {
    let content = read_bytes(path)?;
    let mut decompressed = vec![];
    let mut gz = GzDecoder::new(&content[..]);
    gz.read_to_end(&mut decompressed)?;

    Ok(serde_json::from_slice(&decompressed)?)
}

pub fn save_to_file(path: &Path, chunks: &[Chunk], root_dir: &Path) -> Result<(), Error> {
    let mut tfidf = Vec::with_capacity(chunks.len());

    for chunk in chunks.iter() {
        tfidf.push(ProcessedDoc::new(chunk.uid.clone(), &chunk.into_tfidf_haystack(root_dir)?));
    }

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
            keyword_in_doc: HashMap::new(),
            docs: vec![],
        }
    }

    pub fn consume(
        &mut self,
        doc_id: DocId,
        processed_doc: &ProcessedDoc,
    ) {

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
            let idf = if self.docs.len() > 1 {
                ((self.docs.len() as f32 + 1.0) / (*self.keyword_in_doc.get(keyword).unwrap_or(&0) as f32 + 1.0)).log2()
            } else {
                1.0
            };

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

// Decisions and their reasons
// 1. It tries to make tokens as fine as possible. e.g. ["gpt", "4o", "mini"] instead of ["gpt-4o-mini"].
//    It makes sense because ragit's reranking is very strong.
//    As long as we can place the desired chunk in a top 10 list, it doesn't matter whether the chunk is at 1st place or 9th place.
// 2. Tokens are write-once, read-forever. It's okay for tokenizer to be expensive.
pub fn tokenize(s: &str) -> Vec<String> {
    let stemmer = Stemmer::create(Algorithm::English);

    // cjk are very easy to detect
    let mut cjk_tokenizer = TokenizerBuilder::default();
    let cjk_tokenizer = cjk_tokenizer.allow_list(
        &[
            Language::Cmn,
            Language::Jpn,
            Language::Kor,
        ],
    ).build();

    let eng_tokens = s.to_ascii_lowercase().split(
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
        move |s| stemmer.stem(s).to_string()
    ).filter(
        |s| s.len() > 0
    ).collect::<Vec<_>>();
    let mut ecjk_tokens = Vec::with_capacity(eng_tokens.len());

    for eng_token in eng_tokens.iter() {
        for cjk_token in cjk_tokenizer.tokenize(eng_token) {
            ecjk_tokens.push(cjk_token.lemma().to_string());
        }
    }

    ecjk_tokens
}

impl Chunk {
    // very naive heuristic
    // 1. `self.title` is very important, so it's included twice
    // 2. `self.file` might have an information.
    // 3. `self.summary` has constraints that are not in `self.data`.
    //     - It has explanations on images
    //     - It's always English
    // 4. Images have to be replaced with its description.
    pub fn into_tfidf_haystack(&self, root_dir: &Path) -> Result<String, Error> {
        let mut data = self.data.clone();

        for image in self.images.iter() {
            let description_at = Index::get_image_path(
                root_dir,
                image,
                "json",
            );
            let j = read_string(&description_at)?;

            let rep_text = match json::parse(&j)? {
                JsonValue::Object(obj) => match (obj.get("extracted_text"), obj.get("explanation")) {
                    (Some(e1), Some(e2)) => format!("<img>{e1}{e2}</img>"),
                    _ => {
                        return Err(Error::BrokenIndex(format!("schema error at {image}.json")));
                    },
                },
                j => {
                    return Err(Error::JsonTypeError {
                        expected: JsonType::Object,
                        got: get_type(&j),
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
            self.file,
            self.title,
            self.title,
            self.summary,
            data,
        ))
    }
}

pub enum UpdateTfidf {
    Generate,
    Remove,
    Nop,
}
