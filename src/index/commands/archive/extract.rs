use super::{BlockType, decompress};
use crate::INDEX_DIR_NAME;
use crate::chunk::{self, Chunk};
use crate::error::Error;
use crate::index::{
    CHUNK_DIR_NAME,
    CONFIG_DIR_NAME,
    ImageDescription,
    IMAGE_DIR_NAME,
    Index,
    INDEX_FILE_NAME,
    LoadMode,
    METADATA_FILE_NAME,
};
use crate::prompts::PROMPT_DIR;
use crate::uid::Uid;
use ragit_fs::{
    WriteMode,
    exists,
    file_size,
    join3,
    join4,
    parent,
    read_bytes_offset,
    remove_dir_all,
    set_extension,
    try_create_dir,
    write_bytes,
    write_string,
};
use ragit_pdl::decode_base64;
use serde_json::Value;
use std::thread;
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Duration;

impl Index {
    pub fn extract_archive(
        root_dir: &str,
        archives: Vec<String>,
        workers: usize,
    ) -> Result<(), Error> {
        if exists(root_dir) {
            return Err(Error::CannotExtractArchive(format!("`{root_dir}` already exists")));
        }

        let workers = init_workers(workers, root_dir);

        match Index::extract_archive_worker(
            root_dir,
            archives,
            &workers,
        ) {
            Ok(()) => Ok(()),
            Err(e) => {
                for worker in workers.iter() {
                    let _ = worker.send(Request::Kill);
                }

                if exists(root_dir) {
                    remove_dir_all(root_dir)?;
                }

                Err(e)
            },
        }
    }

    fn extract_archive_worker(
        root_dir: &str,
        archives: Vec<String>,
        workers: &[Channel],
    ) -> Result<(), Error> {
        let mut round_robin = 0;
        Index::new(root_dir.to_string())?;

        for archive in archives.iter() {
            let archive_size = file_size(archive)?;
            let mut cursor = 0;

            loop {
                let header = read_bytes_offset(archive, cursor, cursor + 5)?;
                let block_type = BlockType::try_from(header[0]).map_err(|_| Error::BrokenArchive(format!("unknown block type: {}", header[0])))?;
                let body_size = ((header[1] as u64) << 24) +
                    ((header[2] as u64) << 16) +
                    ((header[3] as u64) << 8) +
                    header[4] as u64;

                workers[round_robin % workers.len()].send(Request::Extract {
                    block_type,
                    path: archive.to_string(),
                    from: cursor + 5,
                    to: cursor + 5 + body_size,
                }).map_err(|_| Error::MPSCError(String::from("Extract-archive worker hung up.")))?;
                cursor += 5 + body_size;
                round_robin += 1;

                if cursor == archive_size {
                    break;
                }

                else if cursor > archive_size {
                    return Err(Error::BrokenArchive(format!("`{archive}` is broken. cursor: {cursor}, archive_size: {archive_size}")));
                }
            }
        }

        for worker in workers.iter() {
            worker.send(Request::TellMeWhenYouAreDone).map_err(|_| Error::MPSCError(String::from("Extract-archive worker hung up.")))?;
        }

        let mut killed_workers = vec![];

        loop {
            for (worker_id, worker) in workers.iter().enumerate() {
                if killed_workers.contains(&worker_id) {
                    continue;
                }

                match worker.try_recv() {
                    Ok(msg) => match msg {
                        Response::IAmDone => {
                            worker.send(Request::Kill).map_err(|_| Error::MPSCError(String::from("Extract-archive worker hung up.")))?;
                            killed_workers.push(worker_id);
                        },
                        Response::Error(e) => { return Err(e); },
                    },
                    Err(mpsc::TryRecvError::Empty) => {},
                    Err(mpsc::TryRecvError::Disconnected) => {
                        return Err(Error::MPSCError(String::from("Extract-archive worker hung up.")));
                    },
                }
            }

            if killed_workers.len() == workers.len() {
                break;
            }

            thread::sleep(Duration::from_millis(100));
        }

        // it creates file indexes and tfidfs
        let mut index = Index::load(root_dir.to_string(), LoadMode::Minimum)?;
        index.recover()?;

        Ok(())
    }
}

enum Request {
    Extract { block_type: BlockType, path: String, from: u64, to: u64 },
    TellMeWhenYouAreDone,
    Kill,
}

enum Response {
    Error(Error),
    IAmDone,
}

fn event_loop(
    tx_to_main: mpsc::Sender<Response>,
    rx_from_main: mpsc::Receiver<Request>,
    root_dir: String,
) -> Result<(), Error> {
    for msg in rx_from_main {
        match msg {
            Request::Extract { block_type, path, from, to } => {
                let mut bytes = read_bytes_offset(&path, from, to)?;
                bytes = decompress(&bytes)?;

                match block_type {
                    BlockType::Index => {
                        let index = serde_json::from_slice::<Value>(&bytes)?;
                        let index = serde_json::to_vec_pretty(&index)?;

                        write_bytes(
                            &join3(
                                &root_dir,
                                INDEX_DIR_NAME,
                                INDEX_FILE_NAME,
                            )?,
                            &index,
                            WriteMode::CreateOrTruncate,
                        )?;
                    },
                    BlockType::Chunk => {
                        let chunks = serde_json::from_slice::<Vec<Chunk>>(&bytes)?;

                        for chunk in chunks.iter() {
                            let chunk_at = Index::get_uid_path(
                                &root_dir,
                                CHUNK_DIR_NAME,
                                chunk.uid,
                                Some("chunk"),
                            )?;

                            if !exists(&parent(&chunk_at)?) {
                                try_create_dir(&parent(&chunk_at)?)?;
                            }

                            chunk::save_to_file(
                                &chunk_at,
                                &chunk,
                                0,
                                3,
                                &root_dir,
                                false,  // create tfidf
                            )?;
                        }
                    },
                    BlockType::ImageBytes => {
                        let images = serde_json::from_slice::<HashMap<String, String>>(&bytes)?;

                        for (uid, bytes) in images.iter() {
                            let uid = uid.parse::<Uid>()?;
                            let bytes = decode_base64(bytes)?;
                            let image_at = Index::get_uid_path(
                                &root_dir,
                                IMAGE_DIR_NAME,
                                uid,
                                Some("png"),
                            )?;

                            if !exists(&parent(&image_at)?) {
                                try_create_dir(&parent(&image_at)?)?;
                            }

                            write_bytes(
                                &image_at,
                                &bytes,
                                WriteMode::AlwaysCreate,
                            )?;
                        }
                    },
                    BlockType::ImageDesc => {
                        let descs = serde_json::from_slice::<HashMap<String, ImageDescription>>(&bytes)?;

                        for (uid, desc) in descs.iter() {
                            let uid = uid.parse::<Uid>()?;
                            let desc_at = Index::get_uid_path(
                                &root_dir,
                                IMAGE_DIR_NAME,
                                uid,
                                Some("json"),
                            )?;

                            if !exists(&parent(&desc_at)?) {
                                try_create_dir(&parent(&desc_at)?)?;
                            }

                            write_bytes(
                                &desc_at,
                                &serde_json::to_vec_pretty(desc)?,
                                WriteMode::AlwaysCreate,
                            )?;
                        }
                    },
                    BlockType::Meta => {
                        let meta = serde_json::from_slice::<HashMap<String, String>>(&bytes)?;
                        write_bytes(
                            &join3(
                                &root_dir,
                                INDEX_DIR_NAME,
                                METADATA_FILE_NAME,
                            )?,
                            &serde_json::to_vec_pretty(&meta)?,
                            WriteMode::CreateOrTruncate,
                        )?;
                    },
                    BlockType::Prompt => {
                        let prompts = serde_json::from_slice::<HashMap<String, String>>(&bytes)?;

                        for (name, pdl) in prompts.iter() {
                            write_string(
                                &join4(
                                    &root_dir,
                                    INDEX_DIR_NAME,
                                    PROMPT_DIR,
                                    &set_extension(name, "pdl")?,
                                )?,
                                pdl,
                                WriteMode::CreateOrTruncate,
                            )?;
                        }
                    },
                    BlockType::Config => {
                        let configs = serde_json::from_slice::<HashMap<String, Value>>(&bytes)?;

                        for (name, config) in configs.iter() {
                            write_bytes(
                                &join4(
                                    &root_dir,
                                    INDEX_DIR_NAME,
                                    CONFIG_DIR_NAME,
                                    &set_extension(name, "json")?,
                                )?,
                                &serde_json::to_vec_pretty(config)?,
                                WriteMode::CreateOrTruncate,
                            )?;
                        }
                    },
                }
            },
            // mpsc is fifo, right?
            Request::TellMeWhenYouAreDone => {
                tx_to_main.send(Response::IAmDone).map_err(|_| Error::MPSCError(String::from("Failed to send response to main")))?;
            },
            Request::Kill => { break; },
        }
    }

    drop(tx_to_main);
    Ok(())
}

struct Channel {
    tx_from_main: mpsc::Sender<Request>,
    rx_to_main: mpsc::Receiver<Response>,
}

impl Channel {
    pub fn send(&self, msg: Request) -> Result<(), mpsc::SendError<Request>> {
        self.tx_from_main.send(msg)
    }

    pub fn try_recv(&self) -> Result<Response, mpsc::TryRecvError> {
        self.rx_to_main.try_recv()
    }
}

fn init_workers(n: usize, root_dir: &str) -> Vec<Channel> {
    (0..n).map(|_| init_worker(root_dir.to_string())).collect()
}

fn init_worker(root_dir: String) -> Channel {
    let (tx_to_main, rx_to_main) = mpsc::channel();
    let (tx_from_main, rx_from_main) = mpsc::channel();

    thread::spawn(move || match event_loop(
        tx_to_main.clone(),
        rx_from_main,
        root_dir,
    ) {
        Ok(()) => {},
        Err(e) => {
            tx_to_main.send(Response::Error(e)).unwrap();
        },
    });

    Channel {
        rx_to_main, tx_from_main
    }
}
