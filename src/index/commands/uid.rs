use super::Index;
use ragit_fs::{
    file_name,
    parent,
};
use crate::error::Error;
use crate::uid::Uid;
use sha3::{Digest, Sha3_256};

impl Index {
    /// It uses a cached value if exists.
    pub fn calculate_and_save_uid(&mut self) -> Result<Uid, Error> {
        if self.curr_processing_file.is_some() {
            return Err(Error::DirtyKnowledgeBase);
        }

        match self.uid {
            Some(uid) => Ok(uid),
            None => {
                let uid = self.calculate_uid(false  /* force */)?;
                self.uid = Some(uid);
                self.save_to_file()?;
                Ok(uid)
            },
        }
    }

    /// It uses a cached value if exists.
    pub fn calculate_uid(&self, force: bool) -> Result<Uid, Error> {
        match self.uid {
            Some(uid) if !force => Ok(uid),
            _ => {
                let mut uids = vec![];

                for chunk_path in self.get_all_chunk_files()?.iter() {
                    let chunk_uid_prefix = file_name(&parent(chunk_path)?)?;
                    let chunk_uid_suffix = file_name(chunk_path)?;
                    let uid = format!("{chunk_uid_prefix}{chunk_uid_suffix}").parse::<Uid>()?;
                    uids.push(uid);
                }

                for image_path in self.get_all_image_files()?.iter() {
                    let image_uid_prefix = file_name(&parent(image_path)?)?;
                    let image_uid_suffix = file_name(image_path)?;
                    let uid = format!("{image_uid_prefix}{image_uid_suffix}").parse::<Uid>()?;
                    uids.push(uid);

                    let desc = self.get_image_description_by_uid(uid)?;
                    let mut hasher = Sha3_256::new();
                    hasher.update(desc.extracted_text.as_bytes());
                    hasher.update(desc.explanation.as_bytes());
                    uids.push(format!("{:064x}", hasher.finalize()).parse::<Uid>()?);
                }

                for (key, value) in self.get_all_meta()?.iter() {
                    let mut hasher = Sha3_256::new();
                    hasher.update(key.as_bytes());
                    hasher.update(value.as_bytes());
                    uids.push(format!("{:064x}", hasher.finalize()).parse::<Uid>()?);
                }

                // TODO: Uid::KnowledgeBase doesn't count configs and prompts, and I'm not sure whether it's the right choice
                //
                // 1. Let's say Uid::KnowledgeBase counts configs and prompts. Some knowledge-bases are pushed without configs or prompts.
                //    If you download such knowledge-bases, ragit will load user's configs and prompts.
                //    Then the cloned knowledge-base and remote knowledge-base have different uids even though the user didn't modify anything.
                // 2. Let's say Uid::KnowledgeBase doesn't count configs and prompts. A knowledge-base is pushed with configs and prompts. The
                //    author found a serious issue in its prompt and pushed a new version with new prompts. The new knowledge-base still has the
                //    same uid and no one will know that something's ever changed.
                //
                // I think problem 2 is less serious than problem 1, so I chose not to include configs and prompts. Also, it's more consistent
                // with how git creates a commit hash.

                let mut result = Uid::new_knowledge_base(&uids);

                // `index.summary.uid` is the uid of the knowledge-base without the summary.
                // If it matches `result`, the summary is up to date and must be added to the result.
                if let Some(summary) = &self.summary {
                    if summary.uid == result {
                        uids.push(Uid::new_summary(&summary.summary));
                        result = Uid::new_knowledge_base(&uids);
                    }
                }

                Ok(result)
            },
        }
    }

    // When a knowledge-base is edited, its uid has to be invalidated.
    pub(crate) fn reset_uid(
        &mut self,
        save_to_file: bool,
    ) -> Result<(), Error> {
        if self.uid.is_some() {
            self.uid = None;

            if save_to_file {
                self.save_to_file()?;
            }
        }

        Ok(())
    }
}
