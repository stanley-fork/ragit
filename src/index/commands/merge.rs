use super::{Index, erase_lines};
use chrono::Local;
use crate::chunk;
use crate::error::Error;
use crate::index::{CHUNK_DIR_NAME, IIStatus, IMAGE_DIR_NAME, LoadMode};
use crate::uid::{Uid, UidType};
use ragit_fs::{
    copy_file,
    exists,
    join,
    normalize,
    parent,
    try_create_dir,
};
use std::collections::HashSet;

pub type Path = String;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MergeMode {
    /// If two bases have chunks of the same file, it chooses the ones in the merged base.
    Force,

    /// If two bases have chunks of the same file, it chooses the ones in the merging base.
    Ignore,

    /// If two bases have chunks of the same file, it asks the user to choose which.
    Interactive,

    /// If two bases have chunks of the same file, it dies.
    Reject,
}

impl MergeMode {
    pub fn parse_flag(flag: &str) -> Option<Self> {
        match flag {
            "--force" => Some(MergeMode::Force),
            "--ignore" => Some(MergeMode::Ignore),
            "--interactive" => Some(MergeMode::Interactive),
            "--reject" => Some(MergeMode::Reject),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct MergeResult {
    bases: usize,
    added_files: usize,
    overriden_files: usize,
    ignored_files: usize,
    added_images: usize,
    overriden_images: usize,
    ignored_images: usize,
    added_chunks: usize,
    removed_chunks: usize,
}

impl Index {
    // TODO: it only merges chunks that are `ChunkSource::File`. It has to handle `ChunkSource::Chunks`.
    pub fn merge(
        &mut self,
        path: Path,
        prefix: Option<Path>,
        merge_mode: MergeMode,
        quiet: bool,
        dry_run: bool,
    ) -> Result<MergeResult, Error> {
        let mut result = MergeResult::default();
        let mut old_images = HashSet::new();
        let other = Index::load(path, LoadMode::OnlyJson)?;
        let mut has_to_erase_lines = false;

        for (rel_path, uid_other) in other.processed_files.iter() {
            let mut new_file_path = rel_path.clone();
            let mut new_file_uid = *uid_other;

            if let Some(prefix) = &prefix {
                new_file_path = normalize(&join(prefix, rel_path)?)?;
                new_file_uid = Uid::update_file_uid(*uid_other, rel_path, &new_file_path);
            }

            if let Some(uid_self) = self.processed_files.get(&new_file_path) {
                match merge_mode {
                    MergeMode::Force => {},
                    MergeMode::Ignore => {
                        result.ignored_files += 1;
                        continue;
                    },
                    MergeMode::Interactive => {
                        if !ask_merge(
                            self,
                            *uid_self,
                            &other,
                            *uid_other,
                        )? {
                            result.ignored_files += 1;
                            continue;
                        }
                    },
                    MergeMode::Reject => {
                        return Err(Error::MergeConflict(*uid_self));
                    },
                }

                result.overriden_files += 1;
                result.removed_chunks += self.get_chunks_of_file(*uid_self)?.len();
                self.remove_file(
                    new_file_path.clone(),
                    dry_run,
                    false,  // recursive
                    false,  // auto
                    false,  // staged
                    true,   // processed
                )?;
            }

            else {
                result.added_files += 1;
            }

            if !quiet {
                self.render_merge_dashboard(&result, has_to_erase_lines);
                has_to_erase_lines = true;
            }

            let new_chunk_uids = other.get_chunks_of_file(*uid_other)?;

            // `merge` operation changes chunks' paths and that changes chunks' uids
            let mut modified_new_chunk_uids = Vec::with_capacity(new_chunk_uids.len());

            for new_chunk_uid in new_chunk_uids.iter() {
                let mut new_chunk = other.get_chunk_by_uid(*new_chunk_uid)?;
                result.added_chunks += 1;
                new_chunk.source.set_path(new_file_path.clone());
                new_chunk.uid = Uid::new_chunk(&new_chunk);
                modified_new_chunk_uids.push(new_chunk.uid);

                if !dry_run {
                    self.chunk_count += 1;
                }

                for image in new_chunk.images.iter() {
                    let image_self = Index::get_uid_path(
                        &self.root_dir,
                        IMAGE_DIR_NAME,
                        *image,
                        Some("png"),
                    )?;

                    if !exists(&image_self) {
                        let image_self = Index::get_uid_path(
                            &self.root_dir,
                            IMAGE_DIR_NAME,
                            *image,
                            Some("png"),
                        )?;
                        let desc_self = Index::get_uid_path(
                            &self.root_dir,
                            IMAGE_DIR_NAME,
                            *image,
                            Some("json"),
                        )?;
                        let image_other = Index::get_uid_path(
                            &other.root_dir,
                            IMAGE_DIR_NAME,
                            *image,
                            Some("png"),
                        )?;
                        let desc_other = Index::get_uid_path(
                            &other.root_dir,
                            IMAGE_DIR_NAME,
                            *image,
                            Some("json"),
                        )?;
                        let parent = parent(&image_self)?;

                        if !exists(&parent) {
                            try_create_dir(&parent)?;
                        }

                        copy_file(&image_other, &image_self)?;
                        copy_file(&desc_other, &desc_self)?;
                        result.added_images += 1;
                    }

                    else {
                        old_images.insert(*image);
                    }
                }

                // There's a small catch.
                // If `self` and `other` have the same image with different descriptions,
                // the image may or may not be overriden (later by `for old_image in old_images.iter`).
                // but its tfidf file is created before the description is overriden and is looking at
                // the older version of the description.
                if !dry_run {
                    chunk::save_to_file(
                        &Index::get_uid_path(
                            &self.root_dir,
                            CHUNK_DIR_NAME,
                            new_chunk.uid,
                            Some("chunk"),
                        )?,
                        &new_chunk,
                        self.build_config.compression_threshold,
                        self.build_config.compression_level,
                        &self.root_dir,
                        true,  // create tfidf
                    )?;
                }

                if !quiet {
                    self.render_merge_dashboard(&result, has_to_erase_lines);
                }
            }

            if !dry_run {
                self.add_file_index(new_file_uid, &modified_new_chunk_uids)?;
                self.processed_files.insert(new_file_path, new_file_uid);
            }
        }

        for old_image in old_images.iter() {
            match merge_mode {
                MergeMode::Force => {},
                MergeMode::Ignore => {
                    result.ignored_images += 1;
                    continue;
                },
                MergeMode::Interactive => {
                    if !ask_merge(
                        self,
                        *old_image,
                        &other,
                        *old_image,
                    )? {
                        result.ignored_images += 1;
                        continue;
                    }
                },
                MergeMode::Reject => {
                    return Err(Error::MergeConflict(*old_image));
                },
            }

            let image_self = Index::get_uid_path(
                &self.root_dir,
                IMAGE_DIR_NAME,
                *old_image,
                Some("png"),
            )?;
            let desc_self = Index::get_uid_path(
                &self.root_dir,
                IMAGE_DIR_NAME,
                *old_image,
                Some("json"),
            )?;
            let image_other = Index::get_uid_path(
                &other.root_dir,
                IMAGE_DIR_NAME,
                *old_image,
                Some("png"),
            )?;
            let desc_other = Index::get_uid_path(
                &other.root_dir,
                IMAGE_DIR_NAME,
                *old_image,
                Some("json"),
            )?;
            let parent = parent(&image_self)?;

            if !exists(&parent) {
                try_create_dir(&parent)?;
            }

            copy_file(&image_other, &image_self)?;
            copy_file(&desc_other, &desc_self)?;
            result.overriden_images += 1;
        }

        if !dry_run && (result.added_chunks > 0 || result.removed_chunks > 0) && self.ii_status != IIStatus::None {
            self.ii_status = IIStatus::Outdated;
        }

        if !dry_run {
            self.reset_uid(true /* save_to_file */)?;
            self.save_to_file()?;
        }

        Ok(result)
    }

    fn render_merge_dashboard(
        &self,
        result: &MergeResult,
        has_to_erase_lines: bool,
    ) {
        if has_to_erase_lines {
            erase_lines(10);
        }

        println!("---");
        println!("bases complete: {}", result.bases);
        println!("added files: {}", result.added_files);
        println!("overriden files: {}", result.overriden_files);
        println!("ignored files: {}", result.ignored_files);
        println!("added images: {}", result.added_images);
        println!("overriden images: {}", result.overriden_images);
        println!("ignored images: {}", result.ignored_images);
        println!("added chunks: {}", result.added_chunks);
        println!("removed chunks: {}", result.removed_chunks);
    }
}

// TODO: add a testcase for interactive merging
// It's either a file or an image.
fn ask_merge(
    // to be replaced with
    index1: &Index,
    uid1: Uid,

    // to be replacing
    index2: &Index,
    uid2: Uid,
) -> Result<bool, Error> {
    println!("---");

    // TODO: when [issue #4](https://github.com/baehyunsol/ragit/issues/4) is fixed,
    //       reuse the output formats used by `ls-files` and `ls-images`.
    match (uid1.get_uid_type()?, uid2.get_uid_type()?) {
        // both must be processed
        (UidType::File, UidType::File) => {
            let f1 = index1.get_file_schema(None, Some(uid1))?;
            let f2 = index2.get_file_schema(None, Some(uid2))?;
            let now = Local::now().timestamp();

            println!("There are conflicting files in the 2 knowledge-bases: {}", f1.path);
            println!("");
            println!("--- file 1 (to be replaced with) ---");
            println!("generated by: {}", f1.model);
            println!("last updated at: {}", prettify_time_delta(now, f1.last_updated));
            println!("");
            println!("--- file 2 (to be replacing) ---");
            println!("generated by: {}", f2.model);
            println!("last updated at: {}", prettify_time_delta(now, f2.last_updated));
            println!("");
            println!("------");
            println!("Are you going to replace file 1 with file 2?");
            println!("yes (replace)");
            println!("no (keep file 1)");
            Ok(get_yes_no()?)
        },
        (UidType::Image, UidType::Image) => {
            let i1 = index1.get_image_schema(uid1, false)?;
            let i2 = index2.get_image_schema(uid2, false)?;

            println!("There are conflicting images in the 2 knowledge-bases");
            println!("");
            println!("--- image 1 (to be replaced with) ---");
            println!("explanation: {}", i1.explanation);
            println!("extracted text: {}", i1.extracted_text);
            println!("");
            println!("--- image 2 (to be replacing) ---");
            println!("explanation: {}", i2.explanation);
            println!("extracted text: {}", i2.extracted_text);
            println!("");
            println!("------");
            println!("Are you going to replace image 1 with image 2?");
            println!("yes (replace)");
            println!("no (keep image 1)");
            Ok(get_yes_no()?)
        },
        (t1, t2) => Err(Error::Internal(format!("internal error, merging 2 uids where (uid1: {t1:?}, uid2: {t2:?})"))),
    }
}

// TODO: maybe we need `util.rs`
fn prettify_time_delta(now: i64, past: i64) -> String {
    let delta = now - past;
    let diff = match delta.abs() {
        s if s < 60 => format!("{s} seconds"),
        s if s < 3600 => format!("{} minutes {} seconds", s / 60, s % 60),
        s if s < 86400 => format!("{} hours {} minutes", s / 3600, s % 3600 / 60),
        s if s < 30 * 86400 => format!("{} days {} hours", s / 86400, s % 86400 / 3600),
        s if s < 210 * 86400 => format!("{} weeks {} days", s / 604800, s % 604800 / 86400),
        s if s < 30 * 2629742 => format!("{} months", s / 2629742),
        s => format!("{} years", s / 31556908),
    };

    format!(
        "{diff} {}",
        if delta >= 0 { "ago" } else { "later" },
    )
}

fn get_yes_no() -> Result<bool, Error> {
    loop {
        let mut s = String::new();
        std::io::stdin().read_line(&mut s)?;

        match s.get(0..1).map(|s| s.to_ascii_lowercase()) {
            Some(y) if y == "y" => { return Ok(true); },
            Some(n) if n == "n" => { return Ok(false); },
            _ => {
                println!("just say yes or no");
                continue;
            },
        }
    }
}
