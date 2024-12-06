// Inverted Index Implementation
// Inverted Index is still very naive and lacking many features.
//
// 1. You can only build ii from scratch. There's no incremental build.
// 2. You can only remove the entire ii. There's no removing a single file or a chunk.
// 3. If something goes wrong while an ii is building, you have to build it from scratch.

use super::{II_DIR_NAME, Index};
use crate::INDEX_DIR_NAME;
use crate::error::Error;
use crate::uid::{self, Uid};
use ragit_fs::{
    create_dir_all,
    exists,
    is_dir,
    join3,
    parent,
    read_dir,
    remove_dir_all,
};
use sha3::{Digest, Sha3_256};
use std::cmp::Ordering;
use std::collections::HashMap;

pub type Term = String;
pub type Weight = f32;
const AUTO_FLUSH: usize = 65536;  // TODO: make it configurable

#[derive(Default)]
struct IIBuildState {
    total_uid: usize,
    buffer_uid: usize,
    buffer_term: usize,
    buffer_flush: usize,
}

#[derive(Debug)]
pub struct IIStat {}

impl Index {
    pub fn get_search_candidates(
        &self,
        terms: &HashMap<Term, Weight>,
        limit: usize,
    ) -> Result<Vec<Uid>, Error> {
        let mut result = HashMap::new();

        for (term, weight) in terms.iter() {
            let chunk_uids = self.search_ii_by_term(term)?;
            let score = weight * ((self.chunk_count + 1) as f32 / (chunk_uids.len() + 1) as f32).log2();

            for chunk_uid in chunk_uids.iter() {
                match result.get_mut(chunk_uid) {
                    Some(score_) => { *score_ += score; },
                    None => { result.insert(*chunk_uid, score); },
                }
            }
        }

        let mut result = result.into_iter().collect::<Vec<_>>();

        // It has to be sorted in reverse order
        result.sort_by(|(_, score_a), (_, score_b)| score_b.partial_cmp(score_a).unwrap_or(Ordering::Equal));

        if result.len() > limit {
            result = result[..limit].to_vec();
        }

        Ok(result.into_iter().map(|(uid, _)| uid).collect())
    }

    pub fn search_ii_by_term(&self, term: &Term) -> Result<Vec<Uid>, Error> {
        let ii_path = Index::get_ii_path(&self.root_dir, hash(term));

        if exists(&ii_path) {
            Ok(uid::load_from_file(&ii_path)?)
        }

        else {
            Ok(vec![])
        }
    }

    /// NOTE: It's VERY experimental. It's not even idempotent.
    pub fn build_ii(&self) -> Result<(), Error> {
        let mut buffer = HashMap::with_capacity(AUTO_FLUSH);
        let mut state = IIBuildState::default();

        for uid in self.get_all_chunk_uids()? {
            self.update_ii_buffer(&mut buffer, uid)?;
            state.total_uid += 1;
            state.buffer_uid += 1;
            state.buffer_term = buffer.len();
            self.render_ii_build_dashboard(&state);

            if buffer.len() > AUTO_FLUSH {
                self.flush_ii_buffer(buffer)?;
                buffer = HashMap::with_capacity(AUTO_FLUSH);
                state.buffer_uid = 0;
                state.buffer_flush += 1;
            }
        }

        Ok(())
    }

    pub fn reset_ii(&self) -> Result<(), Error> {
        let ii_path = join3(
            &self.root_dir,
            INDEX_DIR_NAME,
            II_DIR_NAME,
        )?;

        for dir in read_dir(&ii_path)? {
            if is_dir(&dir) {
                remove_dir_all(&dir)?;
            }
        }

        Ok(())
    }

    /// For debugging the inverted index.
    pub fn stat_ii(&self) -> Result<IIStat, Error> {
        todo!()
    }

    pub fn is_ii_built(&self) -> bool {
        let ii_path = join3(
            &self.root_dir,
            INDEX_DIR_NAME,
            II_DIR_NAME,
        ).unwrap_or(String::new());
        let entries = read_dir(&ii_path).unwrap_or(vec![]);

        entries.len() > 0
    }

    fn update_ii_buffer(&self, buffer: &mut HashMap<Term, Vec<Uid>>, uid: Uid) -> Result<(), Error> {
        let tfidf = self.get_tfidf_by_chunk_uid(uid)?;

        for term in tfidf.term_frequency.keys() {
            match buffer.get_mut(term) {
                Some(uids) => {
                    uids.push(uid);
                },
                None => {
                    buffer.insert(term.to_string(), vec![uid]);
                },
            }
        }

        Ok(())
    }

    fn flush_ii_buffer(&self, buffer: HashMap<Term, Vec<Uid>>) -> Result<(), Error> {
        for (term, uids) in buffer.into_iter() {
            let term_hash = hash(&term);
            let ii_path = Index::get_ii_path(&self.root_dir, term_hash);
            let parent_path = parent(&ii_path)?;

            if !exists(&parent_path) {
                create_dir_all(&parent_path)?;
            }

            let uids = if exists(&ii_path) {
                let mut prev_uids = uid::load_from_file(&ii_path)?;
                prev_uids.extend(uids);
                prev_uids
            }

            else {
                uids
            };

            uid::save_to_file(&ii_path, &uids)?;
        }

        Ok(())
    }

    fn render_ii_build_dashboard(&self, state: &IIBuildState) {
        clearscreen::clear().expect("failed to clear screen");
        println!("building an inverted index...");
        println!("total uid: {}", state.total_uid);
        println!("buffer uid: {}", state.buffer_uid);
        println!("buffer term: {}", state.buffer_term);
        println!("buffer flush: {}", state.buffer_flush);
    }
}

fn hash(term: &Term) -> String {
    let mut hasher = Sha3_256::new();
    hasher.update(term.as_bytes());
    format!("{:064x}", hasher.finalize())
}
