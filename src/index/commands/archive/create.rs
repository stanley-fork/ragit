use super::{BlockType, compress, erase_lines};
use crate::constant::{INDEX_DIR_NAME, INDEX_FILE_NAME};
use crate::error::Error;
use crate::index::{ii::IIStatus, Index, LoadMode};
use crate::uid::{self, Uid};
use ragit_fs::{
    FileError,
    FileErrorKind,
    WriteMode,
    exists,
    file_name,
    file_size,
    join3,
    parent,
    read_bytes,
    read_dir,
    read_string,
    remove_file,
    set_extension,
    write_bytes,
};
use ragit_pdl::encode_base64;
use regex::Regex;
use serde_json::Map;
use std::thread;
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

// A block is at most 1 MiB before compressed. Does this number has to be configurable?
const BLOCK_SIZE: usize = 1 << 20;

struct Status {
    started_at: Instant,
    block_count: HashMap<BlockType, usize>,
}

impl Index {
    /// It rejects to create an archive if `output` already exists.
    pub fn create_archive(
        &self,
        workers: usize,

        // it tries its best to keep each file smaller than the limit
        // it's in bytes
        size_limit: Option<u64>,

        // path of archive file
        // if `size_limit` is not none, it may generate multiple files
        // if `size_limit` is not none, it adds suffix to all the output paths (even if there's only single file): `{output}-{seq:06}`
        output: String,
        include_configs: bool,
        include_prompts: bool,
        force: bool,
        quiet: bool,
    ) -> Result<(), Error> {
        let workers = init_workers(
            workers,
            &self.root_dir,
            6,  // compression level (TODO: make it configurable)
        );

        let real_output = if size_limit.is_some() {
            format!("{output}-0000")
        } else {
            output.clone()
        };
        let already_exists = exists(&real_output);

        if already_exists && !force {
            return Err(FileError {
                kind: FileErrorKind::AlreadyExists,
                given_path: Some(real_output),
            }.into());
        }

        match self.create_archive_worker(
            &workers,
            size_limit,
            output.clone(),
            include_configs,
            include_prompts,
            quiet,
        ) {
            Ok(()) => Ok(()),
            Err(e) => {
                let tmp_file_name_re = Regex::new(r"__archive_block_\d{6}_\d{6}").unwrap();

                // clean up
                // 1. kill all workers
                // 2. remove result file, if exists
                // 3. remove blocks generated by workers
                for worker in workers.iter() {
                    let _ = worker.send(Request::Kill);
                }

                // very naive and stupid hack: it has to wait until the workers finish
                // writing tmp files, so that it can clean up the tmp files
                // but no one knows how long it would take. so it just sleeps long enough
                // and starts the clean-up
                thread::sleep(Duration::from_millis(500));

                if size_limit.is_some() {
                    for i in 0..10000 {
                        let output_file = format!("{output}-{i:06}");

                        if exists(&output_file) {
                            let _ = remove_file(&output_file);
                        }

                        else {
                            break;
                        }
                    }
                }

                else if exists(&output) {
                    let _ = remove_file(&output);
                }

                for file in read_dir(".", false)? {
                    if let Ok(file) = file_name(&file) {
                        if tmp_file_name_re.is_match(&file) {
                            let _ = remove_file(&file);
                        }
                    }
                }

                Err(e)
            },
        }
    }

    fn create_archive_worker(
        &self,
        workers: &[Channel],

        // it tries its best to keep each file smaller than the limit
        // it's in bytes
        size_limit: Option<u64>,

        // path of archive file
        // if `size_limit` is not none, it may generate multiple files
        // if `size_limit` is not none, it adds suffix to all the output paths (even if there's only single file): `{output}-{seq:06}`
        output: String,
        include_configs: bool,
        include_prompts: bool,
        quiet: bool,
    ) -> Result<(), Error> {
        let mut curr_block = vec![];
        let mut curr_block_size = 0;
        let mut round_robin = 0;
        let mut status = Status {
            started_at: Instant::now(),
            block_count: HashMap::new(),
        };
        let mut has_to_erase_lines = false;

        if let Some(size_limit) = size_limit {
            if size_limit < 4096 {
                return Err(Error::CannotCreateArchive(String::from("If size-limit is too small, it may behave oddly. Size-limit has to be at least 4 KiB.")));
            }
        }

        workers[round_robin % workers.len()].send(Request::Compress(BlockType::Index, vec![])).map_err(|_| Error::MPSCError(String::from("Create-archive worker hung up.")))?;
        round_robin += 1;
        workers[round_robin % workers.len()].send(Request::Compress(BlockType::Meta, vec![])).map_err(|_| Error::MPSCError(String::from("Create-archive worker hung up.")))?;
        round_robin += 1;

        if include_prompts {
            workers[round_robin % workers.len()].send(Request::Compress(BlockType::Prompt, vec![])).map_err(|_| Error::MPSCError(String::from("Create-archive worker hung up.")))?;
            round_robin += 1;
        }

        if include_configs {
            workers[round_robin % workers.len()].send(Request::Compress(BlockType::Config, vec![])).map_err(|_| Error::MPSCError(String::from("Create-archive worker hung up.")))?;
            round_robin += 1;
        }

        for file_index in self.get_all_file_indexes()? {
            for chunk_uid in uid::load_from_file(&file_index)? {
                let chunk = self.get_chunk_by_uid(chunk_uid)?;
                let curr_chunk_size = chunk.get_approx_size();

                if curr_chunk_size + curr_block_size > BLOCK_SIZE {
                    workers[round_robin % workers.len()].send(Request::Compress(BlockType::Chunk, curr_block)).map_err(|_| Error::MPSCError(String::from("Create-archive worker hung up.")))?;
                    curr_block = vec![chunk.uid];
                    curr_block_size = curr_chunk_size;
                    round_robin += 1;
                }

                else {
                    curr_block_size += curr_chunk_size;
                    curr_block.push(chunk.uid);
                }
            }
        }

        if !curr_block.is_empty() {
            workers[round_robin % workers.len()].send(Request::Compress(BlockType::Chunk, curr_block)).map_err(|_| Error::MPSCError(String::from("Create-archive worker hung up.")))?;
            round_robin += 1;
        }

        let mut curr_image_block = vec![];
        let mut curr_image_desc_block = vec![];
        let mut curr_image_block_size = 0;
        let mut curr_image_desc_block_size = 0;

        for image_file in self.get_all_image_files()? {
            let image_uid = Uid::from_prefix_and_suffix(
                &file_name(&parent(&image_file)?)?,
                &file_name(&image_file)?,
            )?;
            let image_bytes_len = file_size(&image_file)?;
            let image_desc_len = file_size(&set_extension(&image_file, "json")?)?;

            if image_bytes_len + curr_image_block_size > BLOCK_SIZE as u64 {
                workers[round_robin % workers.len()].send(Request::Compress(BlockType::ImageBytes, curr_image_block)).map_err(|_| Error::MPSCError(String::from("Create-archive worker hung up.")))?;
                curr_image_block = vec![image_uid];
                curr_image_block_size = image_bytes_len;
                round_robin += 1;
            }

            else {
                curr_image_block_size += image_bytes_len;
                curr_image_block.push(image_uid);
            }

            if image_desc_len + curr_image_desc_block_size > BLOCK_SIZE as u64 {
                workers[round_robin % workers.len()].send(Request::Compress(BlockType::ImageDesc, curr_image_desc_block)).map_err(|_| Error::MPSCError(String::from("Create-archive worker hung up.")))?;
                curr_image_desc_block = vec![image_uid];
                curr_image_desc_block_size = image_desc_len;
                round_robin += 1;
            }

            else {
                curr_image_desc_block_size += image_desc_len;
                curr_image_desc_block.push(image_uid);
            }
        }

        if !curr_image_block.is_empty() {
            workers[round_robin % workers.len()].send(Request::Compress(BlockType::ImageBytes, curr_image_block)).map_err(|_| Error::MPSCError(String::from("Create-archive worker hung up.")))?;
            round_robin += 1;
        }

        if !curr_image_desc_block.is_empty() {
            workers[round_robin % workers.len()].send(Request::Compress(BlockType::ImageDesc, curr_image_desc_block)).map_err(|_| Error::MPSCError(String::from("Create-archive worker hung up.")))?;
        }

        let mut curr_output_size = 0;
        let mut curr_output_seq = 0;
        let mut killed_workers = vec![];

        // assumption: `u64::MAX` is practically infinite
        let size_limit_comp = size_limit.unwrap_or(u64::MAX);
        let mut curr_output_file = if size_limit.is_some() { format!("{output}-{curr_output_seq:06}") } else { output.clone() };
        write_bytes(
            &curr_output_file,
            &[],
            WriteMode::CreateOrTruncate,
        )?;
        let mut splitted_block_index = 0;

        for worker in workers.iter() {
            worker.send(Request::TellMeWhenYouAreDone).map_err(|_| Error::MPSCError(String::from("Create-archive worker hung up.")))?;
        }

        loop {
            if !quiet {
                self.render_archive_create_dashboard(
                    &status,
                    workers.len() - killed_workers.len(),
                    curr_output_seq,
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
                        Response::Compressed(block_type, block_path) => {
                            let block_size = file_size(&block_path)?;

                            match status.block_count.get_mut(&block_type) {
                                Some(n) => { *n += 1; },
                                None => { status.block_count.insert(block_type, 1); },
                            }

                            // a file consists of multiple blocks and a block consists of a header and a body

                            // 1. header
                            // It's always 5 bytes. The first byte tells the type of the block
                            // and the other 4 bytes is the length of the body.
                            write_bytes(
                                &curr_output_file,
                                &[
                                    block_type.to_byte(),
                                    (block_size >> 24) as u8,
                                    ((block_size >> 16) & 0xff) as u8,
                                    ((block_size >> 8) & 0xff) as u8,
                                    (block_size & 0xff) as u8,
                                ],
                                WriteMode::AlwaysAppend,
                            )?;
                            // 2. body
                            write_bytes(
                                &curr_output_file,
                                &read_bytes(&block_path)?,
                                WriteMode::AlwaysAppend,
                            )?;
                            curr_output_size += block_size + 5;

                            if curr_output_size > size_limit_comp {
                                if curr_output_size > size_limit_comp {
                                    // 8 is room for metadata
                                    let approx_split_count = curr_output_size / (size_limit_comp - 8) + 1;
                                    let chunk_size = (curr_output_size / approx_split_count + 1) as usize;
                                    let bytes = read_bytes(&curr_output_file)?;
                                    let split_count = bytes.chunks(chunk_size).count();

                                    for (index, chunk) in bytes.chunks(chunk_size).enumerate() {
                                        write_bytes(
                                            &curr_output_file,
                                            &[
                                                BlockType::Splitted.to_byte(),
                                                (splitted_block_index >> 16) as u8,
                                                ((splitted_block_index >> 8) & 0xff) as u8,
                                                (splitted_block_index & 0xff) as u8,
                                                (index >> 8) as u8,
                                                (index & 0xff) as u8,
                                                (split_count >> 8) as u8,
                                                (split_count & 0xff) as u8,
                                            ],
                                            WriteMode::CreateOrTruncate,
                                        )?;
                                        write_bytes(
                                            &curr_output_file,
                                            chunk,
                                            WriteMode::AlwaysAppend,
                                        )?;
                                        curr_output_seq += 1;
                                        curr_output_file = format!("{output}-{curr_output_seq:06}");
                                    }

                                    curr_output_seq -= 1;
                                    splitted_block_index += 1;
                                }

                                curr_output_size = 0;
                                curr_output_seq += 1;
                                curr_output_file = format!("{output}-{curr_output_seq:06}");
                                write_bytes(
                                    &curr_output_file,
                                    &[],
                                    WriteMode::AlwaysCreate,
                                )?;
                            }

                            remove_file(&block_path)?;
                        },
                        Response::IAmDone => {
                            worker.send(Request::Kill).map_err(|_| Error::MPSCError(String::from("Create-archive worker hung up.")))?;
                            killed_workers.push(worker_id);
                        },
                        Response::Error(e) => {
                            return Err(e);
                        },
                    },
                    Err(mpsc::TryRecvError::Empty) => {},
                    Err(mpsc::TryRecvError::Disconnected) => {
                        return Err(Error::MPSCError(String::from("Create-archive worker hung up.")));
                    },
                }
            }

            if killed_workers.len() == workers.len() {
                break;
            }

            thread::sleep(Duration::from_millis(100));
        }

        if exists(&curr_output_file) && file_size(&curr_output_file)? == 0 {
            remove_file(&curr_output_file)?;
        }

        if !quiet {
            self.render_archive_create_dashboard(
                &status,
                workers.len() - killed_workers.len(),
                curr_output_seq,
                has_to_erase_lines,
            );
        }

        Ok(())
    }

    fn render_archive_create_dashboard(
        &self,
        status: &Status,
        workers: usize,
        output_seq: usize,
        has_to_erase_lines: bool,
    ) {
        if has_to_erase_lines {
            erase_lines(7);
        }

        let elapsed_time = Instant::now().duration_since(status.started_at.clone()).as_secs();
        println!("---");
        println!("elapsed time: {:02}:{:02}", elapsed_time / 60, elapsed_time % 60);
        println!("workers: {workers}");
        println!("archives: {}", output_seq + 1);

        println!("chunk blocks: {}", status.block_count.get(&BlockType::Chunk).unwrap_or(&0));
        println!("image blocks (blob): {}", status.block_count.get(&BlockType::ImageBytes).unwrap_or(&0));
        println!("image blocks (desc): {}", status.block_count.get(&BlockType::ImageDesc).unwrap_or(&0));
    }
}

enum Request {
    Compress(BlockType, Vec<Uid>),
    TellMeWhenYouAreDone,
    Kill,
}

enum Response {
    Compressed(BlockType, String),
    IAmDone,
    Error(Error),
}

fn event_loop(
    tx_to_main: mpsc::Sender<Response>,
    rx_from_main: mpsc::Receiver<Request>,
    root_dir: String,
    worker_id: usize,
    compression_level: u32,
) -> Result<(), Error> {
    let index = Index::load(root_dir, LoadMode::OnlyJson)?;
    let mut seq = 0;

    for msg in rx_from_main {
        match msg {
            Request::Compress(block_type, uids) => {
                let block_data = match block_type {
                    BlockType::Index => {
                        let index_json = read_string(&join3(
                            &index.root_dir,
                            INDEX_DIR_NAME,
                            INDEX_FILE_NAME,
                        )?)?;
                        let mut index = serde_json::from_str::<Index>(&index_json)?;

                        // archive does not include ii
                        index.ii_status = IIStatus::None;

                        let index_json = serde_json::to_vec(&index)?;
                        compress(&index_json, compression_level)?
                    },
                    BlockType::Chunk => {
                        let mut chunks = Vec::with_capacity(uids.len());

                        for uid in uids.iter() {
                            chunks.push(index.get_chunk_by_uid(*uid)?);
                        }

                        let bytes = serde_json::to_vec(&chunks)?;
                        compress(&bytes, compression_level)?
                    },
                    BlockType::ImageBytes => {
                        let mut images = HashMap::with_capacity(uids.len());

                        // I know it's inefficient, ... but it works!
                        for uid in uids.iter() {
                            images.insert(
                                uid.to_string(),
                                encode_base64(&index.get_image_bytes_by_uid(*uid)?),
                            );
                        }

                        let bytes = serde_json::to_vec(&images)?;
                        compress(&bytes, compression_level)?
                    },
                    BlockType::ImageDesc => {
                        let mut descs = HashMap::with_capacity(uids.len());

                        for uid in uids.iter() {
                            descs.insert(uid.to_string(), index.get_image_description_by_uid(*uid)?);
                        }

                        let bytes = serde_json::to_vec(&descs)?;
                        compress(&bytes, compression_level)?
                    },
                    BlockType::Meta => {
                        let meta = index.get_all_meta()?;

                        if meta.is_empty() {
                            vec![]
                        }

                        else {
                            let bytes = serde_json::to_vec(&meta)?;
                            compress(&bytes, compression_level)?
                        }
                    },
                    BlockType::Prompt => {
                        let bytes = serde_json::to_vec(&index.prompts)?;
                        compress(&bytes, compression_level)?
                    },
                    BlockType::Config => {
                        let mut obj = Map::new();
                        obj.insert(String::from("api"), serde_json::to_value(&index.api_config)?);
                        obj.insert(String::from("build"), serde_json::to_value(&index.build_config)?);
                        obj.insert(String::from("query"), serde_json::to_value(&index.query_config)?);
                        let bytes = serde_json::to_vec(&obj)?;
                        compress(&bytes, compression_level)?
                    },
                    BlockType::Splitted { .. } => unreachable!(),
                };

                if !block_data.is_empty() {
                    let block_file_name = format!("__archive_block_{worker_id:06}_{seq:06}");
                    write_bytes(
                        &block_file_name,
                        &block_data,
                        WriteMode::AlwaysCreate,
                    )?;
                    seq += 1;

                    tx_to_main.send(Response::Compressed(block_type, block_file_name)).map_err(|_| Error::MPSCError(String::from("Failed to send response to main")))?;
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

fn init_workers(n: usize, root_dir: &str, compression_level: u32) -> Vec<Channel> {
    (0..n).map(|i| init_worker(i, root_dir.to_string(), compression_level)).collect()
}

fn init_worker(worker_id: usize, root_dir: String, compression_level: u32) -> Channel {
    let (tx_to_main, rx_to_main) = mpsc::channel();
    let (tx_from_main, rx_from_main) = mpsc::channel();

    thread::spawn(move || match event_loop(
        tx_to_main.clone(),
        rx_from_main,
        root_dir,
        worker_id,
        compression_level,
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
