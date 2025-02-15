use super::Index;
use crate::chunk::{self, CHUNK_DIR_NAME};
use crate::error::Error;
use crate::index::{
    ChunkBuildInfo,
    FileReader,
    IIStatus,
    IMAGE_DIR_NAME,
    LoadMode,
};
use crate::uid::Uid;
use ragit_api::record::Record;
use ragit_fs::{
    WriteMode,
    create_dir_all,
    exists,
    parent,
    write_bytes,
};
use sha3::{Digest, Sha3_256};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

impl Index {
    pub async fn build(&mut self, workers: usize) -> Result<(), Error> {
        let mut remaining_chunks = 0;
        let started_at = Instant::now();
        println!("counting chunks...");

        for file in self.staged_files.iter() {
            let real_path = Index::get_data_path(
                &self.root_dir,
                file,
            )?;
            let mut fd = FileReader::new(file.to_string(), real_path, self.build_config.clone())?;

            while fd.can_generate_chunk() {
                remaining_chunks += 1;
                fd.next_chunk()?;
            }
        }

        let mut workers = init_workers(workers, self.root_dir.clone());

        match self.build_worker(&mut workers, remaining_chunks, started_at) {
            Ok(()) => Ok(()),
            Err(e) => {
                for worker in workers.iter_mut() {
                    let _ = worker.send(Request::Kill);
                }

                Err(e)
            },
        }
    }

    fn build_worker(
        &mut self,
        workers: &mut Vec<Channel>,
        mut remaining_chunks: usize,
        started_at: Instant,
    ) -> Result<(), Error> {
        let mut killed_workers = vec![];
        let mut staged_files = self.staged_files.clone();
        let mut completed_files = vec![];
        let mut buffered_chunk_count = 0;
        let mut flush_count = 0;

        // HashMap<file, HashMap<index in file, chunk uid>>
        let mut buffer: HashMap<String, HashMap<usize, Uid>> = HashMap::new();

        for (worker_index, worker) in workers.iter_mut().enumerate() {
            if let Some(file) = staged_files.pop() {
                // Previously, all the builds were in serial and this field tells
                // which file the index is building. When something goes wrong, ragit
                // reads this field and clean up garbages. Now, all the builds are in
                // parallel and there's no such thing like `curr_processing_file`. But
                // we still need to tell whether something went wrong while building
                // and this field does that. If it's `Some(_)`, something's wrong and
                // clean-up has to be done.
                self.curr_processing_file = Some(String::new());

                buffer.insert(file.clone(), HashMap::new());
                worker.send(Request::BuildChunks { file }).map_err(|_| Error::MPSCError(String::from("Build worker hung up")))?;
            }

            else {
                worker.send(Request::Kill).map_err(|_| Error::MPSCError(String::from("Build worker hung up.")))?;
                killed_workers.push(worker_index);
            }
        }

        loop {
            self.render_build_dashboard(
                &buffer,
                &completed_files,
                started_at.clone(),
                flush_count,
                remaining_chunks,
            )?;

            for (worker_index, worker) in workers.iter_mut().enumerate() {
                if killed_workers.contains(&worker_index) {
                    continue;
                }

                match worker.try_recv() {
                    Ok(msg) => match msg {
                        Response::ChunkComplete { file, chunk_uid, index } => {
                            buffered_chunk_count += 1;
                            remaining_chunks -= 1;

                            match buffer.get_mut(&file) {
                                Some(chunks) => {
                                    if let Some(prev_uid) = chunks.insert(index, chunk_uid) {
                                        return Err(Error::Internal(format!("{}th chunk of {file} is created twice: {prev_uid}, {chunk_uid}", index + 1)));
                                    }
                                },
                                None => {
                                    let mut chunks = HashMap::new();
                                    chunks.insert(index, chunk_uid);
                                    buffer.insert(file, chunks);
                                },
                            }
                        },
                        Response::FileComplete { file, chunk_count } => {
                            match buffer.get(&file) {
                                Some(chunks) => {
                                    if chunks.len() != chunk_count {
                                        return Err(Error::Internal(format!("Some chunks in `{file}` are missing: expected {chunk_count} chunks, got only {} chunks.", chunks.len())));
                                    }

                                    for i in 0..chunk_count {
                                        if !chunks.contains_key(&i) {
                                            return Err(Error::Internal(format!(
                                                "{} chunk of `{file}` is missing.",
                                                match i {
                                                    0 => String::from("1st"),
                                                    1 => String::from("2nd"),
                                                    2 => String::from("3rd"),
                                                    n => format!("{}th", n + 1),
                                                },
                                            )));
                                        }
                                    }
                                },
                                None if chunk_count != 0 => {
                                    return Err(Error::Internal(format!("Some chunks in `{file}` are missing: expected {chunk_count} chunks, got no chunks.")));
                                },
                                None => {},
                            }

                            if let Some(file) = staged_files.pop() {
                                buffer.insert(file.clone(), HashMap::new());
                                worker.send(Request::BuildChunks { file }).map_err(|_| Error::MPSCError(String::from("Build worker hung up.")))?;
                            }

                            else {
                                worker.send(Request::Kill).map_err(|_| Error::MPSCError(String::from("Build worker hung up.")))?;
                                killed_workers.push(worker_index);
                            }

                            completed_files.push(file);
                        },
                        Response::Error(e) => {
                            return Err(e);
                        },
                    },
                    Err(mpsc::error::TryRecvError::Empty) => {},
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        if !killed_workers.contains(&worker_index) {
                            return Err(Error::MPSCError(String::from("Build worker hung up.")));
                        }
                    },
                }
            }

            // It flushes and commits 8 files at once.
            // TODO: this number has to be configurable
            if completed_files.len() > 8 || killed_workers.len() == workers.len() {
                self.staged_files = self.staged_files.iter().filter(
                    |staged_file| !completed_files.contains(staged_file)
                ).map(
                    |staged_file| staged_file.to_string()
                ).collect();
                let mut ii_buffer = HashMap::new();

                for file in completed_files.iter() {
                    let real_path = Index::get_data_path(
                        &self.root_dir,
                        file,
                    )?;

                    if self.processed_files.contains_key(file) {
                        self.remove_file(real_path.clone(), false)?;
                    }

                    let file_uid = Uid::new_file(&self.root_dir, &real_path)?;
                    let mut chunk_uids = buffer.get(file).unwrap().iter().map(
                        |(index, uid)| (*index, *uid)
                    ).collect::<Vec<_>>();
                    chunk_uids.sort_by_key(|(index, _)| *index);
                    let chunk_uids = chunk_uids.into_iter().map(|(_, chunk_uid)| chunk_uid).collect::<Vec<_>>();
                    self.add_file_index(file_uid, &chunk_uids)?;
                    self.processed_files.insert(file.to_string(), file_uid);

                    match self.ii_status {
                        IIStatus::Complete => {
                            for chunk_uid in chunk_uids.iter() {
                                self.update_ii_buffer(&mut ii_buffer, *chunk_uid)?;
                            }
                        },
                        IIStatus::Ongoing(_)
                        | IIStatus::Outdated => {
                            self.ii_status = IIStatus::Outdated;
                        },
                        IIStatus::None => {},
                    }

                    buffer.remove(file);
                }

                self.chunk_count += buffered_chunk_count;
                self.flush_ii_buffer(ii_buffer)?;
                self.save_to_file()?;

                buffered_chunk_count = 0;
                completed_files = vec![];
                flush_count += 1;

                if killed_workers.len() == workers.len() {
                    self.render_build_dashboard(
                        &buffer,
                        &completed_files,
                        started_at.clone(),
                        flush_count,
                        remaining_chunks,
                    )?;
                    break;
                }
            }

            std::thread::sleep(Duration::from_millis(100));
        }

        Ok(())
    }

    fn render_build_dashboard(
        &self,
        buffer: &HashMap<String, HashMap<usize, Uid>>,
        completed_files: &[String],
        started_at: Instant,
        flush_count: usize,
        remaining_chunks: usize,
    ) -> Result<(), Error> {
        clearscreen::clear().expect("failed to clear screen");
        let elapsed_time = Instant::now().duration_since(started_at).as_secs();
        let mut curr_processing_files = vec![];

        for file in buffer.keys() {
            if !completed_files.contains(file) {
                curr_processing_files.push(format!("`{file}`"));
            }
        }

        println!("elapsed time: {:02}:{:02}", elapsed_time / 60, elapsed_time % 60);
        println!("staged files: {}, processed files: {}", self.staged_files.len(), self.processed_files.len());
        println!("remaining chunks (approx): {remaining_chunks}");
        println!("committed chunks: {}", self.chunk_count);
        println!(
            "currently processing files: {}",
            if curr_processing_files.is_empty() {
                String::from("null")
            } else {
                curr_processing_files.join(", ")
            },
        );
        println!(
            "buffered files: {}, buffered chunks: {}",
            buffer.len(),
            buffer.values().map(|h| h.len()).sum::<usize>(),
        );
        println!("flush count: {flush_count}");
        println!("model: {}", self.api_config.model);

        let api_records = self.api_config.get_api_usage("create_chunk_from")?;
        let mut input_tokens = 0;
        let mut output_tokens = 0;
        let mut input_cost = 0;
        let mut output_cost = 0;

        for Record { input, output, input_weight, output_weight, .. } in api_records.iter() {
            input_tokens += input;
            output_tokens += output;
            input_cost += input * input_weight;
            output_cost += output * output_weight;
        }

        println!(
            "input tokens: {input_tokens} ({:.3}$), output tokens: {output_tokens} ({:.3}$)",
            input_cost as f64 / 1_000_000_000.0,
            output_cost as f64 / 1_000_000_000.0,
        );
        Ok(())
    }
}

async fn event_loop(
    tx_to_main: mpsc::UnboundedSender<Response>,
    mut rx_from_main: mpsc::UnboundedReceiver<Request>,
    root_dir: String,
) -> Result<(), Error> {
    // Each process requires an instance of `Index`, but I found
    // it too difficult to send the instance via mpsc channels.
    // So I'm just instantiating new ones here.
    // TODO: is there a better way?
    let index = Index::load(
        root_dir,
        LoadMode::OnlyJson,
    )?;
    let prompt = index.get_prompt("summarize")?;
    let mut hasher = Sha3_256::new();
    hasher.update(prompt.as_bytes());
    let prompt_hash = hasher.finalize();
    let prompt_hash = format!("{prompt_hash:064x}");

    while let Some(msg) = rx_from_main.recv().await {
        match msg {
            Request::BuildChunks { file } => {
                let real_path = Index::get_data_path(
                    &index.root_dir,
                    &file,
                )?;
                let mut fd = FileReader::new(
                    file.clone(),
                    real_path.clone(),
                    index.build_config.clone(),
                )?;
                let build_info = ChunkBuildInfo::new(
                    fd.file_reader_key(),
                    prompt_hash.clone(),
                    index.api_config.model.clone(),
                );
                let mut index_in_file = 0;
                let mut previous_summary = None;

                while fd.can_generate_chunk() {
                    let new_chunk = fd.generate_chunk(
                        &index,
                        build_info.clone(),
                        previous_summary.clone(),
                        index_in_file,
                    ).await?;
                    previous_summary = Some((new_chunk.clone(), (&new_chunk).into()));
                    let new_chunk_uid = new_chunk.uid;
                    let new_chunk_path = Index::get_uid_path(
                        &index.root_dir,
                        CHUNK_DIR_NAME,
                        new_chunk_uid,
                        Some("chunk"),
                    )?;

                    for (uid, bytes) in fd.images.iter() {
                        let image_path = Index::get_uid_path(
                            &index.root_dir,
                            IMAGE_DIR_NAME,
                            *uid,
                            Some("png"),
                        )?;
                        let parent_path = parent(&image_path)?;
    
                        if !exists(&parent_path) {
                            create_dir_all(&parent_path)?;
                        }
    
                        write_bytes(
                            &image_path,
                            &bytes,
                            WriteMode::Atomic,
                        )?;
                        index.add_image_description(*uid).await?;
                    }

                    chunk::save_to_file(
                        &new_chunk_path,
                        &new_chunk,
                        index.build_config.compression_threshold,
                        index.build_config.compression_level,
                        &index.root_dir,
                    )?;
                    tx_to_main.send(Response::ChunkComplete {
                        file: file.clone(),
                        index: index_in_file,
                        chunk_uid: new_chunk_uid,
                    }).map_err(|_| Error::MPSCError(String::from("Failed to send response to main")))?;
                    index_in_file += 1;
                }

                tx_to_main.send(Response::FileComplete {
                    file,
                    chunk_count: index_in_file,
                }).map_err(|_| Error::MPSCError(String::from("Failed to send response to main")))?;
            },
            Request::Kill => { break; },
        }
    }

    drop(tx_to_main);
    Ok(())
}

#[derive(Debug)]
enum Request {
    BuildChunks { file: String },
    Kill,
}

#[derive(Debug)]
enum Response {
    FileComplete { file: String, chunk_count: usize },
    ChunkComplete { file: String, index: usize, chunk_uid: Uid },
    Error(Error),
}

struct Channel {
    tx_from_main: mpsc::UnboundedSender<Request>,
    rx_to_main: mpsc::UnboundedReceiver<Response>,
}

impl Channel {
    pub fn send(&self, msg: Request) -> Result<(), mpsc::error::SendError<Request>> {
        self.tx_from_main.send(msg)
    }

    pub fn try_recv(&mut self) -> Result<Response, mpsc::error::TryRecvError> {
        self.rx_to_main.try_recv()
    }
}

fn init_workers(n: usize, root_dir: String) -> Vec<Channel> {
    (0..n).map(|_| init_worker(root_dir.clone())).collect()
}

fn init_worker(root_dir: String) -> Channel {
    let (tx_to_main, rx_to_main) = mpsc::unbounded_channel();
    let (tx_from_main, rx_from_main) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        match event_loop(
            tx_to_main.clone(),
            rx_from_main,
            root_dir.clone(),
        ).await {
            Ok(_) => {},
            Err(e) => {
                tx_to_main.send(Response::Error(e)).unwrap();
            },
        }
    });

    Channel {
        rx_to_main, tx_from_main
    }
}
