use super::Index;
use crate::chunk;
use crate::error::Error;
use crate::index::LoadMode;
use crate::uid::Uid;
use ragit_fs::{
    join,
    normalize,
};

pub type Path = String;

pub enum MergeMode {
    Override,
    Ignore,
    Interactive,
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
    pub fn merge(
        &mut self,
        path: Path,
        prefix: Option<Path>,
        merge_mode: MergeMode,
        quiet: bool,
    ) -> Result<MergeResult, Error> {
        let mut result = MergeResult::default();
        let other = Index::load(path, LoadMode::OnlyJson)?;

        for (rel_path, uid_other) in other.processed_files.iter() {
            let mut new_file_path = rel_path.clone();

            if let Some(prefix) = &prefix {
                new_file_path = normalize(&join(prefix, rel_path)?)?;
            }

            if let Some(uid_self) = self.processed_files.get(&new_file_path) {
                match merge_mode {
                    MergeMode::Override => {},
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
                }

                result.overriden_files += 1;
                result.removed_chunks += self.get_chunks_of_file(*uid_self)?.len();
                self.remove_file(new_file_path.clone())?;
            }

            else {
                result.added_files += 1;
            }

            if !quiet {
                self.render_merge_dashboard(&result);
            }

            let new_chunk_uids = other.get_chunks_of_file(*uid_other)?;

            for new_chunk_uid in new_chunk_uids.iter() {
                let mut new_chunk = other.get_chunk_by_uid(*new_chunk_uid)?;
                result.added_chunks += 1;
                self.chunk_count += 1;

                // TODO: By modifying `.file`,
                //       `Uid::new_chunk(&new_chunk) != new_chunk.uid` anymore...
                //       It wouldn't cause any problem, but is a bit ugly...
                new_chunk.file = new_file_path.clone();

                for image in new_chunk.images.iter() {
                    // TODO: merge images
                }

                chunk::save_to_file(
                    &Index::get_chunk_path(
                        &self.root_dir,
                        *new_chunk_uid,
                    ),
                    &new_chunk,
                    self.build_config.compression_threshold,
                    self.build_config.compression_level,
                    &self.root_dir,
                )?;

                if !quiet {
                    self.render_merge_dashboard(&result);
                }
            }

            self.add_file_index(*uid_other, &new_chunk_uids)?;
            self.processed_files.insert(new_file_path, *uid_other);
        }

        Ok(result)
    }

    fn render_merge_dashboard(&self, result: &MergeResult) {
        clearscreen::clear().expect("failed to clear screen");
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

fn ask_merge(
    index1: &Index,
    uid1: Uid,
    index2: &Index,
    uid2: Uid,
) -> Result<bool, Error> {
    todo!()
}
