// Inverted Index Implementation
// Inverted Index is still very naive and lacking many features.
//
// 1. You can only build ii from scratch. There's no incremental build.
// 2. You can only remove the entire ii. There's no removing a single file or a chunk.
// 3. If something goes wrong while an ii is building, you have to build it from scratch.

use super::Index;
use crate::constant::{II_DIR_NAME, INDEX_DIR_NAME};
use crate::error::Error;
use crate::uid::{self, Uid, UidWriteMode};
use ragit_fs::{
    exists,
    file_name,
    is_dir,
    join,
    join3,
    parent,
    read_dir,
    remove_dir_all,
    try_create_dir,
};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

pub type Term = String;
pub type Weight = f32;
const AUTO_FLUSH: usize = 65536;  // TODO: make it configurable

// It takes too long to iterate all the terms and chunks.
const CHECK_II_LIMIT: usize = 512;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum IIStatus {
    /// Initial state. There's no ii at all.
    None,

    /// ii used to be `Complete` or `Ongoing`, but there're added or removed chunks.
    Outdated,

    /// ii is built and is usable.
    Complete,

    /// ii-building is still going on. `ii-build` commands will
    /// start from this uid. `ii-build` ALWAYS processes chunks in
    /// uid order.
    Ongoing(Uid),
}

#[derive(Default)]
struct IIBuildState {
    total_uid: usize,
    buffer_uid: usize,
    buffer_term: usize,
    buffer_flush: usize,
}

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

    /// Very naive way of incremental ii-build.
    /// It works only when `self.ii_status` is `IIStatus::Complete`.
    pub fn add_chunk_to_ii(&mut self, uid: Uid) -> Result<(), Error> {
        if self.ii_status == IIStatus::Complete {
            let mut buffer = HashMap::new();
            self.update_ii_buffer(&mut buffer, uid)?;
            self.flush_ii_buffer(buffer)?;
            Ok(())
        }

        else {
            Err(Error::CannotUpdateII(self.ii_status))
        }
    }

    pub fn build_ii(&mut self, quiet: bool) -> Result<(), Error> {
        match self.ii_status {
            IIStatus::None => {},
            IIStatus::Complete => {
                return Ok(());
            },
            // TODO: resuming `Ongoing` ii-build is not implemented yet
            IIStatus::Outdated | IIStatus::Ongoing(_) => {
                self.reset_ii()?;
            },
        }

        let mut buffer = HashMap::with_capacity(AUTO_FLUSH);
        let mut state = IIBuildState::default();
        let mut uid_check_point = None;

        for uid in self.get_all_chunk_uids()? {
            if uid_check_point.is_none() {
                uid_check_point = Some(uid);
            }

            self.update_ii_buffer(&mut buffer, uid)?;
            state.total_uid += 1;
            state.buffer_uid += 1;
            state.buffer_term = buffer.len();

            if !quiet {
                self.render_ii_build_dashboard(&state);
            }

            if buffer.len() > AUTO_FLUSH {
                self.ii_status = IIStatus::Ongoing(uid_check_point.unwrap());
                uid_check_point = None;
                self.save_to_file()?;

                self.flush_ii_buffer(buffer)?;
                buffer = HashMap::with_capacity(AUTO_FLUSH);
                state.buffer_uid = 0;
                state.buffer_flush += 1;
            }
        }

        if !buffer.is_empty() {
            self.flush_ii_buffer(buffer)?;
        }

        if !quiet {
            self.render_ii_build_dashboard(&state);
        }

        self.ii_status = IIStatus::Complete;
        self.save_to_file()?;
        Ok(())
    }

    pub fn reset_ii(&mut self) -> Result<(), Error> {
        let ii_path = join3(
            &self.root_dir,
            INDEX_DIR_NAME,
            II_DIR_NAME,
        )?;

        for dir in read_dir(&ii_path, false)? {
            if is_dir(&dir) {
                remove_dir_all(&dir)?;
            }
        }

        self.ii_status = IIStatus::None;
        self.save_to_file()?;
        Ok(())
    }

    pub fn check_ii(&self) -> Result<(), Error> {
        let mut term_hash_map: HashMap<String, String> = HashMap::with_capacity(1024);
        let mut from_ii: HashMap<String, Vec<Uid>> = HashMap::with_capacity(1024);
        let mut from_tfidf: HashMap<String, Vec<Uid>> = HashMap::with_capacity(1024);

        'outer: for internal in read_dir(&join3(
            &self.root_dir,
            INDEX_DIR_NAME,
            II_DIR_NAME,
        )?, false)? {
            let prefix = file_name(&internal)?;

            for ii_path in read_dir(&internal, false)? {
                let suffix = file_name(&ii_path)?;
                let term_hash = format!("{prefix}{suffix}");
                let uids = uid::load_from_file(&ii_path)?;
                from_ii.insert(term_hash, uids);

                // It takes too long to iterate all the terms...
                if from_ii.len() == CHECK_II_LIMIT {
                    break 'outer;
                }
            }
        }

        for (uid_index, uid) in self.get_all_chunk_uids()?.into_iter().enumerate() {
            let tfidf = self.get_tfidf_by_chunk_uid(uid)?;

            for term in tfidf.term_frequency.keys() {
                let term_hash = hash(term);

                if from_ii.contains_key(&term_hash) {
                    term_hash_map.insert(term_hash.to_string(), term.to_string());

                    match from_tfidf.get_mut(&term_hash) {
                        Some(uids) => { uids.push(uid) },
                        None => { from_tfidf.insert(term_hash.clone(), vec![uid]); },
                    }
                }

                // it takes too long to iterate all the chunks...
                if uid_index < CHECK_II_LIMIT {
                    let prefix = term_hash.get(0..2).unwrap().to_string();
                    let suffix = term_hash.get(2..).unwrap().to_string();
                    let ii_at = join(
                        &join(
                            &self.root_dir,
                            INDEX_DIR_NAME,
                        )?,
                        &join3(
                            II_DIR_NAME,
                            &prefix,
                            &suffix,
                        )?,
                    )?;

                    if exists(&ii_at) {
                        let ii_uids = uid::load_from_file(&ii_at)?;

                        if !ii_uids.contains(&uid) {
                            return Err(Error::BrokenII(format!("`{term}` is in `{uid}`, but not in ii.")));
                        }
                    }

                    else {
                        return Err(Error::BrokenII(format!("`{term}` is in `{uid}`, but not in ii.")));
                    }
                }
            }
        }

        for (term_hash, term) in term_hash_map.iter() {
            match from_ii.get(term_hash) {
                Some(uids_from_ii) => {
                    let uids_from_ii = uids_from_ii.iter().map(
                        |uid| *uid
                    ).collect::<HashSet<_>>();
                    let uids_from_tfidf = from_tfidf.get(term_hash).unwrap().iter().map(
                        |uid| *uid
                    ).collect::<HashSet<_>>();

                    for uid in uids_from_ii.iter() {
                        if !uids_from_tfidf.contains(uid) {
                            return Err(Error::BrokenII(format!("ii says `{uid}` contains `{term}`, but its tfidf doesn't.")));
                        }
                    }

                    for uid in uids_from_tfidf.iter() {
                        if !uids_from_ii.contains(uid) {
                            return Err(Error::BrokenII(format!("`{term}` is in `{uid}`, but not in ii.")));
                        }
                    }
                },
                // if `from_ii.len() < CHECK_II_LIMIT`, `from_ii` contains all the terms
                None if from_ii.len() < CHECK_II_LIMIT => {
                    return Err(Error::BrokenII(format!("`{term}` is in the knowledge-base, but not in ii.")));
                },
                _ => {},
            }
        }

        for term_hash in from_ii.keys() {
            if !term_hash_map.contains_key(term_hash) {
                return Err(Error::BrokenII(format!("ii has a term_hash `{term_hash}`, but it's not found in actual tfidf files.")));
            }
        }

        Ok(())
    }

    pub fn is_ii_built(&self) -> bool {
        self.ii_status == IIStatus::Complete
    }

    pub(crate) fn update_ii_buffer(&self, buffer: &mut HashMap<Term, Vec<Uid>>, uid: Uid) -> Result<(), Error> {
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

    pub(crate) fn flush_ii_buffer(&self, buffer: HashMap<Term, Vec<Uid>>) -> Result<(), Error> {
        for (term, uids) in buffer.into_iter() {
            let term_hash = hash(&term);
            let ii_path = Index::get_ii_path(&self.root_dir, term_hash);
            let parent_path = parent(&ii_path)?;

            if !exists(&parent_path) {
                try_create_dir(&parent_path)?;
            }

            let uids = if exists(&ii_path) {
                let mut prev_uids = uid::load_from_file(&ii_path)?;
                prev_uids.extend(uids);
                prev_uids
            }

            else {
                uids
            };

            uid::save_to_file(&ii_path, &uids, UidWriteMode::Compact)?;
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
