use crate::agent::{
    Action as AgentAction,
    FileTree,
};
use crate::error::Error;
use crate::index::Index;
use crate::uid::Uid;
use ragit_fs::extension;
use ragit_pdl::Schema;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Summary {
    /// Uid of the knowledge-base *before* this summary is added.
    /// Adding a summary to the knowledge-base alters the uid of the base,
    /// but it's okay because we can get the previous uid by subtracting
    /// the uid of the summary.
    pub uid: Uid,
    pub summary: String,

    // TODO: how about adding more metadata to summary?
    //       I have to decide this before 0.4.2.
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SummaryMode {
    Force,
    Cached,
}

impl SummaryMode {
    pub fn parse_flag(flag: &str) -> Option<Self> {
        match flag {
            "--force" => Some(SummaryMode::Force),
            "--cached" => Some(SummaryMode::Cached),
            _ => None,
        }
    }
}

const SUMMARY_PROMPT: &str = "Give me a summary of the knowledge-base. First, you have to tell me what the knowledge-base is made from. It can be a source code of a program, bunch of documentation files, news articles, papers, or whatever. Then, tell me a brief summary of the contents. If it's a source code, you have to tell me what the program is about and how it's implemented. If it's document files, tell me what it's describing. Lastly, tell me how the files are structured. Please make sure that the summary has less than 1000 characters.";

impl Index {
    pub async fn summary(
        &mut self,
        mode: Option<SummaryMode>,
    ) -> Result<Option<String>, Error> {
        match mode {
            Some(SummaryMode::Force) => {},
            Some(SummaryMode::Cached)
            | None => {
                self.calculate_and_save_uid()?;

                match self.get_summary() {
                    Some(summary) => {
                        return Ok(Some(summary.to_string()));
                    },
                    None if mode == Some(SummaryMode::Cached) => {
                        return Ok(None);
                    },
                    _ => {},
                }
            },
        }

        let summary = if self.chunk_count > 0 || !self.get_all_meta()?.is_empty() {
            let actions = AgentAction::all_actions();

            self.agent(
                SUMMARY_PROMPT,
                self.get_rough_summary()?,  // initial context
                actions,
                Some(Schema::string_length_between(None, Some(1000))),
                true,  // hide summary
            ).await?.response
        } else {
            String::from("This is an empty knowledge-base.")
        };

        // We have to make sure that `self.calculate_uid` calculates uid
        // without any summary.
        self.summary = None;
        self.uid = None;

        self.summary = Some(Summary {
            uid: self.calculate_uid(false  /* force */)?,
            summary,
        });

        // Now it'll update the uid with the summary
        self.calculate_and_save_uid()?;
        Ok(Some(self.summary.clone().unwrap().summary))
    }

    pub fn set_summary(&mut self, summary: &str) -> Result<(), Error> {
        // We have to make sure that `self.calculate_uid` calculates uid
        // without any summary.
        self.summary = None;
        self.uid = None;

        self.summary = Some(Summary {
            uid: self.calculate_uid(false  /* force */)?,
            summary: summary.to_string(),
        });

        // Now it'll update the uid with the summary
        self.calculate_and_save_uid()?;
        Ok(())
    }

    /// This is a read-only version of `Index::summary`.
    /// It returns summary only if there's both summary
    /// and uid and the summary is up-to-date.
    pub fn get_summary(&self) -> Option<&str> {
        match (&self.summary, self.uid) {
            (Some(summary), Some(uid)) => {
                // `Summary` has a uid of the knowledge-base without the summary.
                // So if we add the uid of the summary to it, we must get the
                // current uid of the knowledge-base. Otherwise, the knowledge-base
                // has been edited and the summary is not valid anymore.
                let uid_without_summary = summary.uid;
                let summary_uid = Uid::new_summary(&summary.summary);

                if (summary_uid + uid_without_summary).clear_metadata() == uid.clear_metadata()
                && (uid_without_summary.get_data_size() + 1) & 0xffff_ffff == uid.get_data_size() {
                    Some(&summary.summary)
                }

                else {
                    None
                }
            },
            _ => None,
        }
    }

    pub fn remove_summary(&mut self) -> Result<(), Error> {
        if self.summary.is_some() {
            self.summary = None;
            self.uid = None;
            self.calculate_and_save_uid()?;
        }

        Ok(())
    }

    // This is an initial context of the summary agent.
    pub(crate) fn get_rough_summary(&self) -> Result<String, Error> {
        // HashMap<extension, number of chunks>
        // It counts chunks instead of files because files have variable lengths,
        // but chunks have a length limit.
        let mut count_by_extension = HashMap::new();

        // It's not always equal to `self.chunk_count` because some chunks are not
        // from a file.
        let mut file_chunk_count = 0;

        let mut file_tree = FileTree::root();
        let mut char_len = 0;
        let mut image_uids = HashSet::new();

        for (file, uid) in self.processed_files.iter() {
            let chunks = self.get_chunks_of_file(*uid)?;
            let extension = extension(file)?
                .map(|e| format!(".{e}"))
                .unwrap_or_else(|| String::from("no extension"));
            file_tree.insert(file);
            file_chunk_count += chunks.len();

            match count_by_extension.get_mut(&extension) {
                Some(n) => { *n += chunks.len() },
                _ => { count_by_extension.insert(extension, chunks.len()); },
            }

            for chunk in chunks.iter() {
                let chunk = self.get_chunk_by_uid(*chunk)?;
                char_len += chunk.char_len;

                for image in chunk.images.iter() {
                    image_uids.insert(*image);
                }
            }
        }

        let metadata_len = self.get_all_meta()?.len();
        let mut count_by_extension = count_by_extension.into_iter().collect::<Vec<_>>();
        count_by_extension.sort_by_key(|(_, count)| *count);

        let common_extensions = match count_by_extension.len() {
            0 => String::from(""),
            1 => format!("All the files have the same extension: `{}`", count_by_extension[0].0),
            2 => format!(
                "The files' extension is either `{}` ({:.3} %) or `{}` ({:.3} %)",
                count_by_extension[0].0, count_by_extension[0].1 as f64 * 100.0 / file_chunk_count as f64,
                count_by_extension[1].0, count_by_extension[1].1 as f64 * 100.0 / file_chunk_count as f64,
            ),
            _ => format!(
                "The most common extensions of the files are `{}` ({:.3} %), `{}` ({:.3} %) and `{}` ({:.3} %)",
                count_by_extension[0].0, count_by_extension[0].1 as f64 * 100.0 / file_chunk_count as f64,
                count_by_extension[1].0, count_by_extension[1].1 as f64 * 100.0 / file_chunk_count as f64,
                count_by_extension[2].0, count_by_extension[2].1 as f64 * 100.0 / file_chunk_count as f64,
            ),
        };

        Ok(format!(
            "This knowledge-base consists of {} files ({} characters of text and {} images). {}\nBelow is the list of the files and directories in the knowledge-base.\n\n{}{}",
            self.processed_files.len(),
            char_len,
            image_uids.len(),
            common_extensions,
            file_tree.render(),
            if metadata_len > 0 { format!("\n\nThe knowledge-base has a key-value store for metadata, and it has {metadata_len} keys.") } else { String::new() },
        ))
    }
}
