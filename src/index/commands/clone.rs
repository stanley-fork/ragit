use super::{
    Index,
    erase_lines,
    request_binary_file,
    request_json_file,
};
use super::archive::BlockType;
use crate::constant::{ARCHIVE_DIR_NAME, INDEX_DIR_NAME};
use crate::error::Error;
use crate::index::LoadMode;
use ragit_fs::{
    FileError,
    FileErrorKind,
    WriteMode,
    create_dir,
    exists,
    file_name,
    join,
    join3,
    remove_dir_all,
    rename,
    write_bytes,
};
use reqwest::Url;
use std::collections::HashMap;
use std::time::Instant;

impl Index {
    pub async fn clone(
        url: String,
        repo_name: Option<String>,
        ii: bool,
        quiet: bool,
    ) -> Result<HashMap<BlockType, usize>, Error> {
        let repo_name = repo_name.unwrap_or_else(|| infer_repo_name_from_url(&url));
        let mut archive_tmp_files_at = String::from("archives");
        let mut seq = 0;

        while exists(&archive_tmp_files_at) {
            archive_tmp_files_at = format!("archives-{seq:06}");
            seq += 1;
        }

        create_dir(&archive_tmp_files_at)?;

        if exists(&repo_name) {
            return Err(FileError {
                kind: FileErrorKind::AlreadyExists,
                given_path: Some(repo_name),
            }.into());
        }

        match Index::clone_worker(
            url,
            repo_name.clone(),
            &archive_tmp_files_at,
            ii,
            quiet,
        ).await {
            Ok(result) => Ok(result),
            Err(e) => {
                let _ = remove_dir_all(&archive_tmp_files_at);
                let _ = remove_dir_all(&repo_name);
                Err(e)
            },
        }
    }

    // It first downloads archive files to `archive_tmp_files_at`, and extract the files.
    // After the extraction, a knowledge-base is created. It moves the archive files in `archive_tmp_files_at`
    // to `{repo_name}/.ragit/archives` and removes `archive_tmp_files_at`.
    async fn clone_worker(
        mut url: String,
        repo_name: String,
        archive_tmp_files_at: &str,
        ii: bool,
        quiet: bool,
    ) -> Result<HashMap<BlockType, usize>, Error> {
        if !url.ends_with("/") {
            url = format!("{url}/");
        }

        let url = Url::parse(&url)?;
        let started_at = Instant::now();
        let archive_list_url = url.join("archive-list/")?;
        let archive_list = request_json_file(archive_list_url.as_str()).await?;
        let archive_list = serde_json::from_value::<Vec<String>>(archive_list)?;

        if archive_list.is_empty() && !quiet {
            eprintln!("You appeared to have cloned an empty knowledge-base.");
        }

        let mut archive_files = vec![];
        let mut downloaded_bytes = 0;
        let mut has_to_erase_lines = false;

        for (index, archive) in archive_list.iter().enumerate() {
            let archive_url = url.join("archive/")?.join(archive)?;
            let archive_blob = request_binary_file(archive_url.as_str()).await?;
            downloaded_bytes += archive_blob.len();

            if !quiet {
                Index::render_clone_dashboard(
                    started_at.clone(),
                    index + 1,
                    archive_list.len(),
                    downloaded_bytes,
                    has_to_erase_lines,
                );
                has_to_erase_lines = true;
            }

            let archive_file = join(
                archive_tmp_files_at,
                archive,
            )?;

            write_bytes(
                &archive_file,
                &archive_blob,
                WriteMode::CreateOrTruncate,
            )?;
            archive_files.push(archive_file);
        }

        let block_types = Index::extract_archive(
            &repo_name,
            archive_files.clone(),
            4,      // workers  // TODO: make it configurable
            false,  // force
            ii,
            quiet,
        )?;
        let archives_in_base = join3(
            &repo_name,
            INDEX_DIR_NAME,
            ARCHIVE_DIR_NAME,
        )?;

        if !exists(&archives_in_base) {
            create_dir(&archives_in_base)?;
        }

        for archive_file in archive_files.iter() {
            rename(archive_file, &join(&archives_in_base, &file_name(archive_file)?)?)?;
        }

        remove_dir_all(archive_tmp_files_at)?;
        let mut index = Index::load(repo_name, LoadMode::Minimum)?;
        index.repo_url = Some(url.to_string());
        index.save_to_file()?;
        Ok(block_types)
    }

    fn render_clone_dashboard(
        started_at: Instant,
        completed_downloads: usize,
        total_downloads: usize,
        downloaded_bytes: usize,
        has_to_erase_lines: bool,
    ) {
        if has_to_erase_lines {
            erase_lines(3);
        }

        let elapsed_time = Instant::now().duration_since(started_at).as_millis() as usize;
        let elapsed_sec = elapsed_time / 1000;
        let bytes_per_second = if elapsed_time < 100 || completed_downloads < 3 {
            0
        } else {
            downloaded_bytes * 1000 / elapsed_time
        };

        println!("---");
        println!("elapsed time: {:02}:{:02}", elapsed_sec / 60, elapsed_sec % 60);
        println!(
            "fetching archives: {completed_downloads}/{total_downloads}, {} | {}",
            if downloaded_bytes < 1024 {
                format!("{downloaded_bytes} bytes")
            } else if downloaded_bytes < 1048576 {
                format!("{}.{} KiB", downloaded_bytes >> 10, (downloaded_bytes & 0x3ff) / 102)
            } else {
                format!("{}.{} MiB", downloaded_bytes >> 20, (downloaded_bytes & 0xfffff) / 104857)
            },
            if bytes_per_second == 0 {
                String::from("??? KiB/s")
            } else if bytes_per_second < 1048576 {
                format!("{}.{} KiB/s", bytes_per_second >> 10, (bytes_per_second & 0x3ff) / 102)
            } else {
                format!("{}.{} MiB/s", bytes_per_second >> 20, (bytes_per_second & 0xfffff) / 104857)
            },
        );
    }
}

// TODO: it's too naive
fn infer_repo_name_from_url(url: &str) -> String {
    // This function doesn't need any error-handling
    // because if any of these fail, `Index::clone_worker()`
    // would also fail and there's an error handler for
    // `Index::clone_worker()`.
    match Url::parse(url) {
        Ok(url) => match url.path_segments() {
            Some(paths) => match paths.last() {
                Some(name) => name.to_string(),
                _ => String::from("_"),
            },
            _ => String::from("_"),
        },
        _ => String::from("_"),
    }
}
