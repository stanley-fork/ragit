use super::Index;
use super::archive::BlockType;
use crate::constant::{
    CHUNK_DIR_NAME,
    CONFIG_DIR_NAME,
    FILE_INDEX_DIR_NAME,
    IMAGE_DIR_NAME,
    INDEX_DIR_NAME,
    INDEX_FILE_NAME,
    METADATA_FILE_NAME,
    PROMPT_DIR_NAME,
};
use crate::error::Error;
use ragit_fs::{
    exists,
    join,
    remove_dir_all,
    rename,
};
use reqwest::Url;

pub enum PullResult {
    PulledArchives,
    AlreadyUpToDate,
}

impl Index {
    // TODO: `include_configs` and `include_prompts` are not thoroughly tested yet
    pub async fn pull(
        &self,
        include_configs: bool,
        include_prompts: bool,
        quiet: bool,
    ) -> Result<PullResult, Error> {
        let Some(repo_url) = self.repo_url.clone() else {
            return Err(Error::NoRemoteToPullFrom);
        };

        // compare remote uid and local uid. if they're the same do nothing
        let mut url = Url::parse(&repo_url)?;
        url.set_port(Some(41127)).map_err(|_| Error::RequestFailure {
            context: Some(String::from("pull")),
            code: None,
            url: url.as_str().to_string(),
        })?;
        let get_uid_url = url.join("uid")?;

        match self.get_uid("pull", get_uid_url.as_str()).await {
            Ok(remote_uid) => {
                let self_uid = self.calculate_uid()?;

                if remote_uid == self_uid {
                    return Ok(PullResult::AlreadyUpToDate);
                }
            },
            Err(e) => {
                if !quiet {
                    eprintln!("Failed to get {get_uid_url}: {e:?}");
                }
            },
        }

        let mut tmp_no = 0;
        let mut tmp_clone_dir = format!("tmp-clone-dir-{tmp_no}");

        while exists(&tmp_clone_dir) {
            tmp_no += 1;
            tmp_clone_dir = format!("tmp-clone-dir-{tmp_no}");
        }

        let cloned_blocks = Index::clone(repo_url, Some(tmp_clone_dir.clone()), quiet).await?;
        let cloned_configs = cloned_blocks.get(&BlockType::Config).map(|n| *n).unwrap_or(0) > 1;
        let cloned_prompts = cloned_blocks.get(&BlockType::Prompt).map(|n| *n).unwrap_or(0) > 1;

        let curr_index_dir = join(
            &self.root_dir,
            INDEX_DIR_NAME,
        )?;
        let new_index_dir = join(
            &tmp_clone_dir,
            INDEX_DIR_NAME,
        )?;

        // If power goes down while moving `.ragit/files/`, you can run `rag check --recover` to recover
        // If power goes down while moving `chunks/`, `images/` or `index.json`, you can run `rag pull` again to recover
        // If power goes down while moving `meta.json`... you cannot tell whether something's wrong or not. That's a problem
        for dir in [
            FILE_INDEX_DIR_NAME,
            CHUNK_DIR_NAME,
            IMAGE_DIR_NAME,
        ].iter() {
            remove_dir_all(&join(&curr_index_dir, dir)?)?;
            rename(
                &join(&new_index_dir, dir)?,
                &join(&curr_index_dir, dir)?,
            )?;
        }

        rename(
            &join(&new_index_dir, INDEX_FILE_NAME)?,
            &join(&curr_index_dir, INDEX_FILE_NAME)?,
        )?;

        if exists(METADATA_FILE_NAME) {
            rename(
                &join(&new_index_dir, METADATA_FILE_NAME)?,
                &join(&curr_index_dir, METADATA_FILE_NAME)?,
            )?;
        }

        if include_configs && cloned_configs {
            remove_dir_all(&join(&curr_index_dir, CONFIG_DIR_NAME)?)?;
            rename(
                &join(&new_index_dir, CONFIG_DIR_NAME)?,
                &join(&curr_index_dir, CONFIG_DIR_NAME)?,
            )?;
        }

        if include_prompts && cloned_prompts {
            remove_dir_all(&join(&curr_index_dir, PROMPT_DIR_NAME)?)?;
            rename(
                &join(&new_index_dir, PROMPT_DIR_NAME)?,
                &join(&curr_index_dir, PROMPT_DIR_NAME)?,
            )?;
        }

        Ok(PullResult::PulledArchives)
    }
}
