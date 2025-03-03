use super::{Leaf, Tree, generate_tree};
use async_recursion::async_recursion;
use chrono::Local;
use crate::chunk::{self, Chunk, ChunkBuildInfo, ChunkSchema, ChunkSource};
use crate::constant::CHUNK_DIR_NAME;
use crate::error::Error;
use crate::index::Index;
use crate::uid::Uid;
use ragit_api::Request;
use ragit_pdl::{Pdl, parse_pdl};
use sha3::{Digest, Sha3_256};
use std::collections::HashMap;

impl Index {
    // If a summary of the file already exists, it just loads the
    // summary instead of creating a new one.
    pub async fn summary_file(&mut self, file: Uid) -> Result<Chunk, Error> {
        let chunks = self.get_chunks_of_file(file)?;
        let mut tree = generate_tree(chunks.len(), 10);  // TODO: make the limit configurable
        let mut chunk_by_index = HashMap::with_capacity(chunks.len());
        tree.mark_range(&mut 0);

        for chunk_uid in chunks.iter() {
            let chunk = self.get_chunk_by_uid(*chunk_uid)?;
            chunk_by_index.insert(chunk.source.unwrap_index(), chunk.uid);
        }

        let result = self.summary_tree(&tree, &chunk_by_index, false).await?;
        self.get_chunk_by_uid(result)
    }

    // It returns the uid of the summary chunk.
    #[async_recursion(?Send)]
    pub async fn summary_tree(&mut self, tree: &Tree, chunk_by_index: &HashMap<usize, Uid>, dry_run: bool) -> Result<Uid, Error> {
        let (new_uid, chunk_uids) = match tree {
            Tree::Leaf(Leaf::Range { from, to }) => {
                let chunk_uids = (*from..*to).map(|index| *chunk_by_index.get(&index).unwrap()).collect::<Vec<_>>();
                let new_uid = Uid::new_group(&chunk_uids);

                // create a new chunk only when necessary
                if !self.check_chunk_by_uid(new_uid) && !dry_run {
                    (new_uid, chunk_uids)
                }

                else {
                    return Ok(new_uid);
                }
            },
            Tree::Leaf(_) => unreachable!(),
            Tree::Node(nodes) => {
                let mut chunk_uids = Vec::with_capacity(nodes.len());

                for node in nodes.iter() {
                    let chunk_uid = self.summary_tree(node, chunk_by_index, dry_run).await?;
                    chunk_uids.push(chunk_uid);
                }

                let new_uid = Uid::new_group(&chunk_uids);

                // create a new chunk only when necessary
                if !self.check_chunk_by_uid(new_uid) && !dry_run {
                    (new_uid, chunk_uids)
                }

                else {
                    return Ok(new_uid);
                }
            },
        };

        if dry_run {
            Ok(new_uid)
        }

        else {
            let mut chunks = Vec::with_capacity(chunk_uids.len());

            for chunk_uid in chunk_uids.iter() {
                chunks.push(self.get_chunk_by_uid(*chunk_uid)?);
            }

            let new_chunk = self.summary_chunks(&chunks).await?;
            chunk::save_to_file(
                &Index::get_uid_path(
                    &self.root_dir,
                    CHUNK_DIR_NAME,
                    new_chunk.uid,
                    Some("chunk"),
                )?,
                &new_chunk,
                self.build_config.compression_threshold,
                self.build_config.compression_level,
                &self.root_dir,
                true,  // create tfidf
            )?;
            self.chunk_count += 1;
            Ok(new_chunk.uid)
        }
    }

    // It assumes that the order of the chunks has a meaning.
    pub async fn summary_chunks(&self, chunks: &[Chunk]) -> Result<Chunk, Error> {
        match chunks.len() {
            0 => todo!(),
            1 => Ok(Chunk {
                data: chunks[0].data.clone(),
                images: chunks[0].images.clone(),
                char_len: chunks[0].char_len,
                image_count: chunks[0].image_count,
                title: chunks[0].title.clone(),
                summary: chunks[0].summary.clone(),
                source: ChunkSource::Chunks { uids: vec![chunks[0].uid] },
                searchable: false,
                uid: Uid::new_group(&[chunks[0].uid]),
                build_info: chunks[0].build_info.clone(),
                timestamp: Local::now().timestamp(),
            }),
            _ => {
                let chunk_uids = chunks.iter().map(|chunk| chunk.uid).collect::<Vec<_>>();
                let data = chunks.iter().map(
                    |chunk| format!("title: {}\nsummary: {}", chunk.title, chunk.summary)
                ).collect::<Vec<_>>().join("\n\n");
                let mut context = tera::Context::new();
                let prompt = self.get_prompt("summarize_chunks")?;
                let mut hasher = Sha3_256::new();
                hasher.update(prompt.as_bytes());
                let prompt_hash = hasher.finalize();
                let prompt_hash = format!("{prompt_hash:064x}");

                context.insert("data", &data);
                context.insert("max_summary_len", &self.build_config.max_summary_len);
                context.insert("min_summary_len", &self.build_config.min_summary_len);
                context.insert("chunks", chunks);

                let Pdl { messages, schema } = parse_pdl(
                    &prompt,
                    &context,
                    "/",  // TODO: `<|media|>` is not supported for this prompt
                    true,
                    true,
                )?;
                let request = Request {
                    messages,
                    model: self.get_model_by_name(&self.api_config.model)?,
                    max_retry: self.api_config.max_retry,
                    sleep_between_retries: self.api_config.sleep_between_retries,
                    timeout: self.api_config.timeout,
                    record_api_usage_at: self.api_config.dump_api_usage_at(&self.root_dir, "summary_chunks"),
                    dump_pdl_at: self.api_config.create_pdl_path(&self.root_dir, "summary_chunks"),
                    dump_json_at: self.api_config.dump_log_at(&self.root_dir),
                    schema,
                    schema_max_try: 3,
                    ..Request::default()
                };
                let response = request.send_and_validate::<ChunkSchema>(ChunkSchema::dummy(
                    &format!(
                        "[{}]",
                        chunks.iter().map(|chunk| format!("{:?}", chunk.title)).collect::<Vec<_>>().join(", "),
                    ),
                    self.build_config.max_summary_len,
                )).await?;

                let mut result = Chunk {
                    data: data.clone(),
                    images: vec![],
                    char_len: data.chars().count(),
                    image_count: 0,
                    title: response.title,
                    summary: response.summary,
                    source: ChunkSource::Chunks { uids: chunk_uids.clone() },
                    searchable: false,
                    uid: Uid::dummy(),
                    build_info: ChunkBuildInfo::new(
                        String::from("chunk_grouper_v0"),
                        prompt_hash,
                        self.api_config.model.clone(),
                    ),
                    timestamp: Local::now().timestamp(),
                };
                result.uid = Uid::new_group(&chunk_uids);
                Ok(result)
            },
        }
    }
}
