use super::{BlockType, compress};
use crate::INDEX_DIR_NAME;
use crate::error::Error;
use crate::index::{Index, INDEX_FILE_NAME, LoadMode};
use crate::uid::{self, Uid};
use ragit_fs::{
    WriteMode,
    exists,
    file_name,
    file_size,
    join3,
    parent,
    read_bytes,
    read_string,
    remove_file,
    set_extension,
    write_bytes,
};
use ragit_pdl::encode_base64;
use serde_json::{Map, Value};
use std::thread;
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Duration;

// A block is at most 1 MiB before compressed. Does this number has to be configurable?
const BLOCK_SIZE: usize = 1 << 20;

impl Index {
    pub fn create_archive(
        &self,
        workers: usize,

        // it tries its best to keep each file smaller than the limit
        // it's in bytes
        size_limit: Option<u64>,

        // path of archive file
        // if `size_limit` is not none, it may generate multiple files
        // if `size_limit` is not none, it adds suffix to all the output paths (even if there's only single file): `{output}-{seq:04}`
        output: String,
        include_configs: bool,
        include_prompts: bool,
    ) -> Result<(), Error> {
        let workers = init_workers(workers, &self.root_dir);

        match self.create_archive_worker(
            &workers,
            size_limit,
            output.clone(),
            include_configs,
            include_prompts,
        ) {
            Ok(()) => Ok(()),
            Err(e) => {
                // clean up
                // 1. kill all workers
                // 2. remove result file, if exists
                for worker in workers.iter() {
                    let _ = worker.send(Request::Kill);
                }

                if size_limit.is_some() {
                    for i in 0..10000 {
                        if exists(&format!("{output}-{i:04}")) {
                            let _ = remove_file(&format!("{output}-{i:04}"));
                        }

                        else {
                            break;
                        }
                    }
                }

                else if exists(&output) {
                    let _ = remove_file(&output);
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
        // if `size_limit` is not none, it adds suffix to all the output paths (even if there's only single file): `{output}-{seq:04}`
        output: String,
        include_configs: bool,
        include_prompts: bool,
    ) -> Result<(), Error> {
        let mut curr_block = vec![];
        let mut curr_block_size = 0;
        let mut round_robin = 0;

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

        let mut curr_output_size = 0;
        let mut curr_output_seq = 0;
        let mut killed_workers = vec![];

        // assumption: `u64::MAX` is practically infinite
        let size_limit_comp = size_limit.unwrap_or(u64::MAX);
        let mut curr_output_file = if size_limit.is_some() { format!("{output}-{curr_output_seq:04}") } else { output.clone() };
        write_bytes(
            &curr_output_file,
            &[],
            WriteMode::AlwaysCreate,
        )?;

        for worker in workers.iter() {
            worker.send(Request::TellMeWhenYouAreDone).map_err(|_| Error::MPSCError(String::from("Create-archive worker hung up.")))?;
        }

        loop {
            for (worker_id, worker) in workers.iter().enumerate() {
                if killed_workers.contains(&worker_id) {
                    continue;
                }

                match worker.try_recv() {
                    Ok(msg) => match msg {
                        Response::Compressed(block_type, block_path) => {
                            let block_size = file_size(&block_path)?;

                            if block_size + curr_output_size > size_limit_comp {
                                curr_output_size = block_size;
                                curr_output_seq += 1;
                                curr_output_file = format!("{output}-{curr_output_seq:04}");
                                write_bytes(
                                    &curr_output_file,
                                    &[],
                                    WriteMode::AlwaysCreate,
                                )?;
                            }

                            else {
                                curr_output_size += block_size;
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

        Ok(())
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
                        // un-prettify index.json, then compress the json
                        let index_json = serde_json::from_str::<Value>(&index_json)?;
                        let index_json = serde_json::to_vec(&index_json)?;
                        compress(&index_json, 3)?
                    },
                    BlockType::Chunk => {
                        let mut chunks = Vec::with_capacity(uids.len());

                        for uid in uids.iter() {
                            chunks.push(index.get_chunk_by_uid(*uid)?);
                        }

                        let bytes = serde_json::to_vec(&chunks)?;
                        compress(&bytes, 3)?
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
                        compress(&bytes, 3)?
                    },
                    BlockType::ImageDesc => {
                        let mut descs = HashMap::with_capacity(uids.len());

                        // TODO: it has to encode uid
                        for uid in uids.iter() {
                            descs.insert(uid.to_string(), index.get_image_description_by_uid(*uid)?);
                        }

                        let bytes = serde_json::to_vec(&descs)?;
                        compress(&bytes, 3)?
                    },
                    BlockType::Meta => {
                        let meta = index.get_all_meta()?;

                        if meta.is_empty() {
                            vec![]
                        }

                        else {
                            let bytes = serde_json::to_vec(&meta)?;
                            compress(&bytes, 3)?
                        }
                    },
                    BlockType::Prompt => {
                        let bytes = serde_json::to_vec(&index.prompts)?;
                        compress(&bytes, 3)?
                    },
                    BlockType::Config => {
                        let mut obj = Map::new();
                        obj.insert(String::from("api"), serde_json::to_value(&index.api_config_raw)?);
                        obj.insert(String::from("build"), serde_json::to_value(&index.build_config)?);
                        obj.insert(String::from("query"), serde_json::to_value(&index.query_config)?);
                        let bytes = serde_json::to_vec(&obj)?;
                        compress(&bytes, 3)?
                    },
                };

                if !block_data.is_empty() {
                    let block_file_name = format!("__archive_block_{worker_id:04}_{seq:04}");
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

fn init_workers(n: usize, root_dir: &str) -> Vec<Channel> {
    (0..n).map(|i| init_worker(i, root_dir.to_string())).collect()
}

fn init_worker(worker_id: usize, root_dir: String) -> Channel {
    let (tx_to_main, rx_to_main) = mpsc::channel();
    let (tx_from_main, rx_from_main) = mpsc::channel();

    thread::spawn(move || match event_loop(
        tx_to_main.clone(),
        rx_from_main,
        root_dir,
        worker_id,
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
