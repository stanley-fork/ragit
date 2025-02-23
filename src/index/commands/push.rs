use crate::INDEX_DIR_NAME;
use crate::error::Error;
use crate::index::{ARCHIVE_DIR_NAME, Index};
use ragit_fs::{
    create_dir,
    exists,
    file_name,
    join,
    join3,
    read_bytes,
    read_dir,
    remove_dir_all,
};
use reqwest::Url;
use std::time::Instant;

impl Index {
    pub async fn push(
        &self,
        mut remote: Option<String>,
        include_configs: bool,
        include_prompts: bool,
    ) -> Result<(), Error> {
        if remote.is_none() {
            remote = self.repo_url.clone();

            if remote.is_none() {
                return Err(Error::CannotPush(String::from("Please specify where to push.")));
            }
        }

        let mut remote = remote.unwrap();

        if !remote.ends_with("/") {
            remote = format!("{remote}/");
        }

        let archives_at = join3(
            &self.root_dir,
            INDEX_DIR_NAME,
            ARCHIVE_DIR_NAME,
        )?;

        if !exists(&archives_at) {
            create_dir(&archives_at)?;
        }

        let started_at = Instant::now();
        let mut uploaded_bytes = 0;
        let mut url = Url::parse(&remote)?;
        url.set_port(Some(41127)).map_err(|_| Error::PushRequestError {
            code: None,
            url: url.as_str().to_string(),
        })?;

        // TODO: I want it to reuse archives from
        // previous runs -> but how do I know whether
        // it's valid?
        if exists(&archives_at) {
            remove_dir_all(&archives_at)?;
            create_dir(&archives_at)?;
        }

        self.create_archive(
            4,  // workers  // TODO: make it configurable
            Some(1 << 19),  // at most 512KiB per file
            join(&archives_at, "ar")?,
            include_configs,
            include_prompts,
            false,
        )?;
        let get_session_id_url = url.join("begin-push")?;
        let session_id = self.get_session_id(get_session_id_url.as_str()).await?;
        let archives = read_dir(&archives_at, false)?;

        for (index, archive) in archives.iter().enumerate() {
            let archive_id = file_name(&archive)?;
            let blob = read_bytes(archive)?;
            uploaded_bytes += blob.len();
            let send_archive_file_url = url.join("archive")?;
            self.send_archive_file(send_archive_file_url.as_str(), &session_id, &archive_id, blob).await?;
            self.render_push_dashboard(
                started_at.clone(),
                index + 1,
                archives.len(),
                uploaded_bytes,
            );
        }

        let finalize_push_url = url.join("finalize-push")?;
        self.finalize_push(finalize_push_url.as_str(), &session_id).await?;
        Ok(())
    }

    async fn get_session_id(&self, url: &str) -> Result<String, Error> {
        let client = reqwest::Client::new();
        let mut client = client.post(url);

        if let Some((username, password)) = self.auth() {
            client = client.basic_auth(username, password);
        }

        let response = client.send().await?;

        if response.status().as_u16() != 200 {
            return Err(Error::PushRequestError {
                code: Some(response.status().as_u16()),
                url: url.to_string(),
            });
        }

        Ok(String::from_utf8(response.bytes().await?.to_vec())?)
    }

    async fn send_archive_file(
        &self,
        url: &str,
        session_id: &str,
        archive_id: &str,
        blob: Vec<u8>,
    ) -> Result<(), Error> {
        let client = reqwest::Client::new();
        let response = client.post(url).multipart(
            reqwest::multipart::Form::new()
                .text("session-id", session_id.to_string())
                .text("archive-id", archive_id.to_string())
                .part("archive", reqwest::multipart::Part::bytes(blob))
        ).send().await?;

        if response.status().as_u16() != 200 {
            return Err(Error::PushRequestError {
                code: Some(response.status().as_u16()),
                url: url.to_string(),
            });
        }

        Ok(())
    }

    async fn finalize_push(&self, url: &str, session_id: &str) -> Result<(), Error> {
        // I don't know much about REST API, but people are saying that GET has to be idempotent.
        // That's why I'm using POST.
        let client = reqwest::Client::new();
        let response = client.post(url).body(session_id.to_string()).send().await?;

        if response.status().as_u16() != 200 {
            return Err(Error::PushRequestError {
                code: Some(response.status().as_u16()),
                url: url.to_string(),
            });
        }

        Ok(())
    }

    fn render_push_dashboard(
        &self,
        started_at: Instant,
        completed_uploads: usize,
        total_uploads: usize,
        uploaded_bytes: usize,
    ) {
        clearscreen::clear().expect("failed to clear screen");
        let elapsed_time = Instant::now().duration_since(started_at).as_millis() as usize;
        let elapsed_sec = elapsed_time / 1000;
        let bytes_per_second = if elapsed_time < 100 || completed_uploads < 3 {
            0
        } else {
            uploaded_bytes * 1000 / elapsed_time
        };

        println!("elapsed time: {:02}:{:02}", elapsed_sec / 60, elapsed_sec % 60);
        println!(
            "uploading archives: {completed_uploads}/{total_uploads}, {} | {}",
            if uploaded_bytes < 1024 {
                format!("{uploaded_bytes} bytes")
            } else if uploaded_bytes < 1048576 {
                format!("{}.{} KiB", uploaded_bytes >> 10, (uploaded_bytes & 0x3ff) / 102)
            } else {
                format!("{}.{} MiB", uploaded_bytes >> 20, (uploaded_bytes & 0xfffff) / 104857)
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
