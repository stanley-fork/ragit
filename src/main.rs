use ragit::{
    AddMode,
    AddResult,
    Chunk,
    Error,
    Index,
    INDEX_DIR_NAME,
    Keywords,
    multi_turn,
    single_turn,
};
use ragit_fs::{
    basename,
    join,
    read_dir,
};
use std::env;
use std::io::Write;

#[tokio::main]
async fn main() {
    match run().await {
        Ok(()) => {},
        Err(e) => {
            // TODO: suggest similar names for some errors
            match e {
                Error::IndexNotFound => {
                    println!("`.rag_index` not found. Make sure that it's a valid ragit repo.");
                },
                Error::InvalidConfigKey(s) => {
                    println!("{s:?} is not a valid key for config.");
                },
                Error::ApiError(g) => match g {
                    ragit_api::Error::InvalidModelKind(m) => {
                        println!(
                            "{m:?} is not a valid name for a chat model. Valid names are\n{}",
                            ragit_api::ChatModel::all_kinds().iter().map(
                                |model| model.to_human_friendly_name()
                            ).collect::<Vec<_>>().join("\n"),
                        );
                    },
                    e => {
                        println!("{e:?}");
                    },
                },
                e => {
                    println!("{e:?}");
                },
            }

            std::process::exit(1);
        },
    }
}

async fn run() -> Result<(), Error> {
    let args = env::args().collect::<Vec<_>>();
    let root_dir = find_root().map_err(|_| Error::IndexNotFound);

    // TODO: nicer cli parsing, error messages and so many more stuffs
    match args.get(1).map(|arg| arg.as_str()) {
        Some("init") => {
            let path = if let Some(path) = args.get(2) { path.to_string() } else { String::from(".") };

            match Index::new(path) {
                Ok(_) => { println!("initialized"); },
                Err(Error::IndexAlreadyExists(_)) => { println!("There already is a knowledge-base here."); },
                Err(e) => { panic!("{e:?}") },
            }
        },
        Some("build") => {
            let mut index = Index::load(root_dir?, false)?;
            index.build_knowledge_base(args.contains(&String::from("--dashboard"))).await?;
        },
        Some("add") => {
            let mut index = Index::load(root_dir?, false)?;
            let (mut added, mut updated, mut ignored) = (0, 0, 0);

            let (flag, files) = if args[2].starts_with("--") {
                (AddMode::from(&args[2]), args[3..].iter())
            } else {
                (AddMode::Ignore, args[2..].iter())
            };

            for path in files {
                let rel_path = Index::get_rel_path(&index.root_dir, path);

                match index.add_file(rel_path, flag) {
                    Ok(AddResult::Added) => { added += 1; },
                    Ok(AddResult::Updated) => { updated += 1; },
                    Ok(AddResult::Ignored) => { ignored += 1; },
                    Err(e) => { panic!("{e:?}"); },
                }
            }

            index.save_to_file()?;
            println!("{added} added files, {updated} updated files, {ignored} ignored files");
        },
        Some("remove") => {
            if args.len() == 2 {
                panic!("<PATH> is not provided.");
            }

            let mut index = Index::load(root_dir?, false)?;

            let remove_count = if args[2] == "--auto" {
                index.remove_auto()?.len()
            }

            else {
                for path in args[2..].iter() {
                    let rel_path = Index::get_rel_path(&index.root_dir, path);
                    index.remove_file(rel_path)?;
                }

                args.len() - 2
            };

            index.save_to_file()?;
            println!("removed {remove_count} files from index");
        },
        Some("merge") => {
            let mut index = Index::load(root_dir?, false)?;
            index.merge(&args[2])?;
            index.save_to_file()?;
        },
        Some("config") => {
            let mut index = Index::load(root_dir?, false)?;

            match args.get(2) {
                Some(arg) if arg == "--set" => {
                    let previous_value = index.set_config_by_key(args[3].clone(), args[4].clone())?;

                    match previous_value {
                        Some(v) => {
                            println!("set `{}`: `{}` -> `{}`", args[3], v, args[4]);
                        },
                        None => {
                            println!("set `{}`: `{}`", args[3], args[4]);
                        },
                    }
                },
                Some(arg) if arg == "--get" => {
                    println!("{}", index.get_config_by_key(args[3].clone())?.dump());
                },
                Some(arg) if arg == "--get-all" => {
                    println!("{}", index.get_all()?.pretty(4));
                },
                Some(arg) => panic!("`{arg}` is not a valid flag"),
                None => panic!("please provide a flag: `--set`, `--get` or `--get-all`"),
            }
        },
        Some("query") => {
            let index = Index::load(root_dir?, true)?;

            match args.get(2) {
                Some(arg) if arg == "-i" || arg == "--interactive" => {
                    let mut conversation = vec![];

                    loop {
                        let mut curr_input = String::new();
                        print!(">>> ");
                        std::io::stdout().flush()?;
                        std::io::stdin().read_line(&mut curr_input)?;
                        conversation.push(curr_input);

                        let result = if conversation.len() == 1 {
                            single_turn(
                                &conversation[0],
                                &index,
                            ).await?
                        } else {
                            multi_turn(
                                conversation.clone(),
                                &index,
                            ).await?
                        };

                        println!("{result}");
                        conversation.push(result);
                    }
                },
                Some(query) => {
                    let result = single_turn(
                        query,
                        &index,
                    ).await?;
                    println!("{result}");
                },
                None => panic!("<QUERY> is not provided."),
            }
        },
        Some("tfidf") => {
            let index = Index::load(root_dir?, true)?;

            match args.get(2) {
                Some(flag) if flag == "--show" => {
                    let processed_doc_data = index.get_tfidf_by_chunk_uid(args[3].clone(), String::from("data"))?;
                    let processed_doc_summary = index.get_tfidf_by_chunk_uid(args[3].clone(), String::from("summary"))?;

                    println!("--- data ---");
                    println!("{}", processed_doc_data.render());
                    println!("--- summary ---");
                    println!("{}", processed_doc_summary.render());
                    return Ok(());
                },
                Some(_) => {},
                None => panic!("Please enter keywords"),
            }

            let keywords = Keywords::from_raw(args[2..].to_vec());
            let tokenized_keywords = keywords.tokenize();
            let tfidf_results = index.run_tfidf(
                keywords,
                vec![],
            )?;
            let uids = tfidf_results.iter().map(|r| r.id.clone()).collect::<Vec<_>>();
            let chunks = index.get_chunks_by_uid(&uids)?;

            println!("search keywords: {:?}", args[2..].to_vec());
            println!("tokenized keywords: {:?}", tokenized_keywords.iter().map(|(token, _)| token).collect::<Vec<_>>());
            println!("found {} results", chunks.len());

            for (tfidf, chunk) in tfidf_results.iter().zip(chunks.iter()) {
                println!("--------------------------");
                println!("score: {}, matched {}", tfidf.score, tfidf.category);
                println!("file: {}", chunk.render_source());
                println!("title: {}", chunk.title);
                println!("summary: {}", chunk.summary);
            }
        },
        Some("ls") => {
            let index = Index::load(root_dir?, true)?;

            match args.get(2) {
                Some(flag) if flag == "--chunks" => {
                    println!("{} chunks", index.chunk_count);
                    let chunks = index.list_chunks(
                        &|_| true,  // no filter
                        &|mut chunk: Chunk| {
                            // it's too big
                            chunk.data = format!("{}", chunk.len());
                            chunk
                        },
                        &|chunk: &Chunk| format!("{}-{:08}", chunk.file, chunk.index),  // sort by file
                    )?;

                    for chunk in chunks.iter() {
                        println!("----------");
                        println!("{}th chunk of {}", chunk.index, chunk.render_source());
                        println!("id: {}", chunk.uid);
                        println!("data len: {}", chunk.data);  // it's mapped above
                        println!("title: {}", chunk.title);
                        println!("summary: {}", chunk.summary);
                    }
                },
                Some(flag) if flag == "--files" => {
                    println!(
                        "{} total files, {} staged files, {} processed files",
                        index.staged_files.len() + index.processed_files.len() + if index.curr_processing_file.is_some() { 1 } else { 0 },
                        index.staged_files.len(),
                        index.processed_files.len() + if index.curr_processing_file.is_some() { 1 } else { 0 },
                    );
                    let files = index.list_files(
                        &|_| true,  // no filter
                        &|f| f,  // no map
                        &|f| f.name.to_string(),
                    );

                    for file in files.iter() {
                        println!("--------");
                        println!("name: {}{}", file.name, if file.is_processed { String::new() } else { String::from(" (not processed yet)") });

                        if file.is_processed {
                            println!("length: {}", file.length);
                            println!("hash: {}", file.hash);
                        }
                    }
                },
                Some(flag) if flag == "--models" => {
                    let models = index.list_models(
                        &|model| model.name != "dummy",  // filter
                        &|model| model,  // no map
                        &|model| model.name.to_string(),
                    );

                    for model in models.iter() {
                        println!("{}", String::from_utf8_lossy(&serde_json::to_vec_pretty(model)?).to_string());
                    }
                },
                Some(flag) => panic!("`{flag}` is an invalid flag"),
                None => panic!("requires a flag"),
            }
        },
        Some("gc") => {
            let index = Index::load(root_dir?, true)?;
            index.gc_logs()?;
            println!("logs removed");
        },
        Some("reset") => {
            let mut index = Index::load(root_dir?, false)?;

            if args[2] == "--hard" {
                index.reset_hard()?;
            }

            else if args[2] == "--soft" {
                index.reset_soft()?;
                index.save_to_file()?;
            }

            else {
                panic!("{:?} is an invalid flag for `reset` command", args[2])
            }
        },
        Some("check") => {
            let index = Index::load(root_dir?, true)?;
            index.check(args.contains(&String::from("--recursive")))?;
            println!("everything is fine!");
        },
        Some("meta") => {
            let index = Index::load(root_dir?, true)?;
            let key = args.get(3);
            let value = args.get(4);

            match args.get(2).map(|arg| arg.as_str()) {
                Some("--get") => {
                    if key.is_none() {
                        panic!("`key` not given");
                    }

                    if let Some(value) = index.meta_get(key.unwrap().to_string())? {
                        println!("{value}");
                    }
                },
                Some("--get-all") => {
                    let all = index.meta_get_all()?;
                    println!("{}", json::JsonValue::from(all).pretty(4));
                },
                Some("--set") => {
                    if key.is_none() {
                        panic!("`key` not given");
                    }

                    if value.is_none() {
                        panic!("`value` not given");
                    }

                    index.meta_set(
                        key.unwrap().to_string(),
                        value.unwrap().to_string(),
                    )?;
                    println!("metadata set");
                },
                Some("--remove") => {
                    if key.is_none() {
                        panic!("`key` not given");
                    }

                    index.meta_remove(key.unwrap().to_string())?;
                    println!("`{}` removed", key.unwrap());
                },
                Some("--remove-all") => {
                    index.meta_remove_all()?;
                    println!("metadata removed");
                },
                f => {
                    let invalid_flag = f.map(|f| f.to_string()).unwrap_or(String::new());
                    panic!("{invalid_flag:?} is an invalid flag for `meta` command.");
                }
            }
        },
        Some(invalid_command) => {
            panic!("{invalid_command:?} is an invalid command.");
        },
        None => {
            // TODO: print help message
        },
    }

    Ok(())
}

// it starts from "." and goes up until it finds ".rag_index"
// you can run git commands anywhere inside a repo, and I want ragit to be like that
fn find_root() -> Result<String, Error> {
    let mut curr = String::from(".");

    loop {
        let curr_files_ = read_dir(&curr)?;
        let mut curr_files = Vec::with_capacity(curr_files_.len());

        for f in curr_files_.iter() {
            curr_files.push(basename(f)?);
        }

        if curr_files.contains(&INDEX_DIR_NAME.to_string()) {
            return Ok(curr);
        }

        curr = join(&curr, "..")?;
    }
}
