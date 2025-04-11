use super::{BlockType, decompress, erase_lines};
use crate::constant::{
    CHUNK_DIR_NAME,
    CONFIG_DIR_NAME,
    IMAGE_DIR_NAME,
    INDEX_DIR_NAME,
    INDEX_FILE_NAME,
    METADATA_FILE_NAME,
    PROMPT_DIR_NAME,
};
use crate::chunk::{self, Chunk};
use crate::error::Error;
use crate::index::{
    ImageDescription,
    Index,
    LoadMode,
};
use crate::uid::Uid;
use ragit_fs::{
    WriteMode,
    exists,
    file_size,
    join,
    join3,
    join4,
    parent,
    read_bytes_offset,
    remove_dir_all,
    remove_file,
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
use std::time::{Duration, Instant};

struct Status {
    started_at: Instant,
    block_count: HashMap<BlockType, usize>,
    block_complete: HashMap<BlockType, usize>,
}

impl Index {
    pub fn extract_archive(
        root_dir: &str,
        archives: Vec<String>,
        workers: usize,
        force: bool,
        quiet: bool,
    ) -> Result<HashMap<BlockType, usize>, Error> {
        if exists(root_dir) {
            if force {
                if exists(&join(root_dir, INDEX_DIR_NAME)?) {
                    remove_dir_all(&join(root_dir, INDEX_DIR_NAME)?)?;
                }
            }

            else {
                return Err(Error::CannotExtractArchive(format!("`{root_dir}` already exists")));
            }
        }

        let workers = init_workers(workers, root_dir);

        match Index::extract_archive_worker(
            root_dir,
            archives,
            &workers,
            quiet,
        ) {
            Ok(result) => Ok(result),
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
        mut archives: Vec<String>,
        workers: &[Channel],
        quiet: bool,
    ) -> Result<HashMap<BlockType, usize>, Error> {
        let mut killed_workers = vec![];
        let mut round_robin = 0;
        let mut status = Status {
            started_at: Instant::now(),
            block_count: HashMap::new(),
            block_complete: HashMap::new(),
        };

        Index::new(root_dir.to_string())?;
        let mut splitted_blocks: HashMap<usize, HashMap<usize, Vec<u8>>> = HashMap::new();
        let mut tmp_files_for_splitted_blocks = vec![];
        let mut has_to_erase_lines = false;

        while let Some(archive) = archives.pop() {
            let archive_size = file_size(&archive)?;
            let mut cursor = 0;

            loop {
                let header = read_bytes_offset(&archive, cursor, cursor + 5)?;

                if header[0] == BlockType::Splitted.to_byte() {
                    match status.block_count.get_mut(&BlockType::Splitted) {
                        Some(n) => { *n += 1; },
                        None => { status.block_count.insert(BlockType::Splitted, 1); },
                    }

                    let header = read_bytes_offset(&archive, cursor, cursor + 8)?;
                    let body = read_bytes_offset(&archive, cursor + 8, file_size(&archive)?)?;
                    let outer_index = ((header[1] as usize) << 16) +
                        ((header[2] as usize) << 8) +
                        header[3] as usize;
                    let inner_index = ((header[4] as usize) << 8) + header[5] as usize;
                    let total_count = ((header[6] as usize) << 8) + header[7] as usize;

                    match splitted_blocks.get_mut(&outer_index) {
                        Some(blocks) => {
                            blocks.insert(inner_index, body);

                            if blocks.len() == total_count {
                                let mut blocks = blocks.iter().map(
                                    |(inner_index, body)| (*inner_index, body.to_vec())
                                ).collect::<Vec<_>>();
                                blocks.sort_by_key(|(inner_index, _)| *inner_index);
                                let blocks = blocks.into_iter().map(
                                    |(_, body)| body
                                ).collect::<Vec<_>>();
                                let tmp_file_for_splitted_blocks = format!("{archive}-splitted-{outer_index:06}");
                                write_bytes(
                                    &tmp_file_for_splitted_blocks,
                                    &blocks.concat(),
                                    WriteMode::AlwaysCreate,
                                )?;
                                splitted_blocks.remove(&outer_index);
                                archives.push(tmp_file_for_splitted_blocks.clone());
                                tmp_files_for_splitted_blocks.push(tmp_file_for_splitted_blocks);
                            }
                        },
                        None => {
                            let mut blocks = HashMap::new();
                            blocks.insert(inner_index, body);
                            splitted_blocks.insert(outer_index, blocks);
                        },
                    }

                    break;
                }

                let block_type = BlockType::try_from(header[0]).map_err(|_| Error::BrokenArchive(format!("unknown block type: {}", header[0])))?;
                let body_size = ((header[1] as u64) << 24) +
                    ((header[2] as u64) << 16) +
                    ((header[3] as u64) << 8) +
                    header[4] as u64;

                match status.block_count.get_mut(&block_type) {
                    Some(n) => { *n += 1; },
                    None => { status.block_count.insert(block_type, 1); },
                }

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

            if !quiet {
                Index::render_archive_extract_dashboard(
                    &status,
                    workers.len() - killed_workers.len(),
                    has_to_erase_lines,
                );
                has_to_erase_lines = true;
            }
        }

        for worker in workers.iter() {
            worker.send(Request::TellMeWhenYouAreDone).map_err(|_| Error::MPSCError(String::from("Extract-archive worker hung up.")))?;
        }

        loop {
            if !quiet {
                Index::render_archive_extract_dashboard(
                    &status,
                    workers.len() - killed_workers.len(),
                    has_to_erase_lines,
                );
                has_to_erase_lines = true;
            }

            for (worker_id, worker) in workers.iter().enumerate() {
                if killed_workers.contains(&worker_id) {
                    continue;
                }

                match worker.try_recv() {
                    Ok(msg) => match msg {
                        Response::Complete(block_type) => {
                            match status.block_complete.get_mut(&block_type) {
                                Some(n) => { *n += 1; },
                                None => { status.block_complete.insert(block_type, 1); },
                            }
                        },
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

        for tmp_file_for_splitted_blocks in tmp_files_for_splitted_blocks.iter() {
            remove_file(tmp_file_for_splitted_blocks)?;
        }

        if !quiet {
            Index::render_archive_extract_dashboard(
                &status,
                workers.len() - killed_workers.len(),
                has_to_erase_lines,
            );
            has_to_erase_lines = true;
        }

        // it creates file indexes and tfidfs
        let mut index = Index::load(root_dir.to_string(), LoadMode::Minimum)?;
        index.recover()?;

        if !quiet {
            Index::render_archive_extract_dashboard(
                &status,
                workers.len() - killed_workers.len(),
                has_to_erase_lines,
            );
        }

        Ok(status.block_count)
    }

    fn render_archive_extract_dashboard(
        status: &Status,
        workers: usize,
        has_to_erase_lines: bool,
    ) {
        if has_to_erase_lines {
            erase_lines(6);
        }

        println!("---");
        let elapsed_time = Instant::now().duration_since(status.started_at.clone()).as_secs();
        println!("elapsed time: {:02}:{:02}", elapsed_time / 60, elapsed_time % 60);
        println!("workers: {workers}");

        println!(
            "chunk blocks: {}/{}",
            status.block_complete.get(&BlockType::Chunk).unwrap_or(&0),
            status.block_count.get(&BlockType::Chunk).unwrap_or(&0),
        );
        println!(
            "image blocks (blob): {}/{}",
            status.block_complete.get(&BlockType::ImageBytes).unwrap_or(&0),
            status.block_count.get(&BlockType::ImageBytes).unwrap_or(&0),
        );
        println!(
            "image blocks (desc): {}/{}",
            status.block_complete.get(&BlockType::ImageDesc).unwrap_or(&0),
            status.block_count.get(&BlockType::ImageDesc).unwrap_or(&0),
        );
    }
}

enum Request {
    Extract { block_type: BlockType, path: String, from: u64, to: u64 },
    TellMeWhenYouAreDone,
    Kill,
}

enum Response {
    Complete(BlockType),
    IAmDone,
    Error(Error),
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
                                    PROMPT_DIR_NAME,
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
                    BlockType::Splitted => unreachable!(),
                }

                tx_to_main.send(Response::Complete(block_type)).map_err(|_| Error::MPSCError(String::from("Failed to send response to main")))?;
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
