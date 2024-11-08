use async_recursion::async_recursion;
use ragit::{
    AddMode,
    AddResult,
    Chunk,
    Error,
    Index,
    INDEX_DIR_NAME,
    Keywords,
    LoadMode,
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

mod cli;
use cli::{
    ArgCount,
    ArgParser,
    ArgType,
};

#[tokio::main]
async fn main() {
    let args = env::args().collect::<Vec<_>>();

    match run(args).await {
        Ok(()) => {},
        Err(e) => {
            // TODO: suggest similar names for some errors
            match e {
                Error::IndexNotFound => {
                    eprintln!("`.ragit/` not found. Make sure that it's a valid ragit repo.");
                },
                Error::InvalidConfigKey(s) => {
                    eprintln!("{s:?} is not a valid key for config.");
                },
                Error::ApiError(g) => match g {
                    ragit_api::Error::InvalidModelKind(m) => {
                        eprintln!(
                            "{m:?} is not a valid name for a chat model. Valid names are\n{}",
                            ragit_api::ChatModel::all_kinds().iter().map(
                                |model| model.to_human_friendly_name()
                            ).collect::<Vec<_>>().join("\n"),
                        );
                    },
                    e => {
                        eprintln!("{e:?}");
                    },
                },
                Error::CliError(e) => {
                    eprintln!("cli error: {e}");
                },
                e => {
                    eprintln!("{e:?}");
                },
            }

            std::process::exit(1);
        },
    }
}

#[async_recursion(?Send)]
async fn run(args: Vec<String>) -> Result<(), Error> {
    let root_dir = find_root().map_err(|_| Error::IndexNotFound);

    match args.get(1).map(|arg| arg.as_str()) {
        Some("add") => {
            let parsed_args = ArgParser::new().flag_with_default(&["--ignore", "--auto", "--force"]).optional_flag(&["--dry-run"]).args(ArgType::Path, ArgCount::Geq(1)).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/add.txt"));
                return Ok(());
            }

            let mut index = Index::load(root_dir?, LoadMode::QuickCheck)?;
            let add_mode = AddMode::parse_flag(&parsed_args.get_flag(0).unwrap()).unwrap();
            let dry_run = parsed_args.get_flag(1).is_some();

            if dry_run {
                return Err(Error::NotImplemented(String::from("rag add --dry-run")));
            }

            let files = parsed_args.get_args();

            let (mut added, mut updated, mut ignored) = (0, 0, 0);

            for path in files.iter() {
                match index.add_file(path, add_mode)? {
                    AddResult::Added => { added += 1; },
                    AddResult::Updated => { updated += 1; },
                    AddResult::Ignored => { ignored += 1; },
                }
            }

            index.save_to_file()?;
            println!("{added} added files, {updated} updated files, {ignored} ignored files");
        },
        Some("build") => {
            let parsed_args = ArgParser::new().optional_flag(&["--dashboard"]).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/build.txt"));
                return Ok(());
            }

            let mut index = Index::load(root_dir?, LoadMode::QuickCheck)?;
            let dashboard = parsed_args.get_flag(0).is_some();
            index.build(dashboard).await?;
        },
        Some("check") => {
            let parsed_args = ArgParser::new().optional_flag(&["--recursive"]).optional_flag(&["--auto-recover"]).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/check.txt"));
                return Ok(());
            }

            let root_dir = root_dir?;
            let recursive = parsed_args.get_flag(0).is_some();
            let auto_recover = parsed_args.get_flag(1).is_some();

            match Index::load(root_dir.clone(), LoadMode::OnlyJson) {
                Ok(mut index) => if index.curr_processing_file.is_some() {
                    index.auto_recover()?;
                    index.save_to_file()?;
                    index.check(recursive)?;
                    println!("auto-recovered from a corrupted knowledge-base");
                } else {
                    match index.check(recursive) {
                        Ok(()) => {
                            println!("everything is fine!");
                        },
                        Err(e) => if auto_recover {
                            index.auto_recover()?;
                            index.save_to_file()?;
                            index.check(recursive)?;
                            println!("auto-recovered from a corrupted knowledge-base");
                        } else {
                            return Err(e);
                        }
                    }
                },
                Err(e) => if auto_recover {
                    let mut index = Index::load(root_dir, LoadMode::Minimum)?;
                    index.auto_recover()?;
                    index.save_to_file()?;
                    index.check(recursive)?;
                    println!("auto-recovered from a corrupted knowledge-base");
                } else {
                    return Err(e);
                },
            }
        },
        Some("clone") => {
            let parsed_args = ArgParser::new().args(ArgType::String, ArgCount::Geq(1)).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/clone.txt"));
                return Ok(());
            }

            let args = parsed_args.get_args();
            Index::clone(
                args[0].clone(),
                args.get(1).map(|s| s.to_string()),
            ).await?;
            return Ok(());
        },
        Some("config") => {
            let parsed_args = ArgParser::new().flag(&["--set", "--get", "--get-all"]).args(ArgType::String, ArgCount::Geq(0)).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/config.txt"));
                return Ok(());
            }

            let mut index = Index::load(root_dir?, LoadMode::OnlyJson)?;

            match parsed_args.get_flag(0).unwrap().as_str() {
                "--set" => {
                    let args = parsed_args.get_args_exact(2)?;
                    let previous_value = index.set_config_by_key(args[0].clone(), args[1].clone())?;

                    match previous_value {
                        Some(v) => {
                            println!("set `{}`: `{}` -> `{}`", args[0].clone(), v, args[1].clone());
                        },
                        None => {
                            println!("set `{}`: `{}`", args[0].clone(), args[1].clone());
                        },
                    }
                },
                "--get" => {
                    let args = parsed_args.get_args_exact(1)?;
                    println!("{}", index.get_config_by_key(args[0].clone())?.dump());
                },
                "--get-all" => {
                    parsed_args.get_args_exact(0)?;  // make sure that there's no dangling args
                    let mut kv = index.get_all_configs()?;
                    kv.sort_by_key(|(k, _)| k.to_string());

                    println!("{}", '{');

                    for (k, v) in kv.iter() {
                        println!("    {k:?}: {},", v.dump());
                    }

                    println!("{}", '}');
                },
                _ => unreachable!(),
            }
        },
        Some("gc") => {
            let parsed_args = ArgParser::new().flag(&["--logs"]).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/gc.txt"));
                return Ok(());
            }

            match parsed_args.get_flag(0).unwrap().as_str() {
                "--logs" => {
                    let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
                    let removed = index.gc_logs()?;
                    println!("removed {removed} log files");
                },
                _ => unreachable!(),
            }
        },
        Some("help") => {
            let parsed_args = ArgParser::new().args(ArgType::Command, ArgCount::Leq(1)).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/help.txt"));
                return Ok(());
            }

            match parsed_args.get_args().get(0).map(|arg| arg.as_str()) {
                Some("chunks") => {
                    println!("{}", include_str!("../docs/chunks.md"));
                },
                Some("config-reference") => {
                    println!("{}", include_str!("../docs/config.md"));
                },
                Some("quick-guide") => {
                    println!("{}", include_str!("../docs/quick_guide.md"));
                },
                Some(command) => {
                    let mut new_args = args.clone();
                    new_args[1] = command.to_string();
                    new_args[2] = String::from("--help");
                    return run(new_args).await;
                },
                None => {
                    println!("{}", include_str!("../docs/commands/general.txt"));
                },
            }
        },
        Some("init") => {
            let parsed_args = ArgParser::new().parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/init.txt"));
                return Ok(());
            }

            match Index::new(String::from(".")) {
                Ok(_) => { println!("initialized"); },
                Err(Error::IndexAlreadyExists(_)) => { println!("There already is a knowledge-base here."); },
                Err(e) => { return Err(e); },
            }
        },
        Some("ls-chunks") => {
            let parsed_args = ArgParser::new().optional_flag(&["--name-only", "--stat-only"]).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/ls-chunks.txt"));
                return Ok(());
            }
            // TODO: impl `--name-only` and `--stat-only`

            let index = Index::load(root_dir?, LoadMode::QuickCheck)?;
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
        Some("ls-files") => {
            let parsed_args = ArgParser::new().optional_flag(&["--name-only", "--stat-only"]).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/ls-files.txt"));
                return Ok(());
            }
            // TODO: impl `--name-only` and `--stat-only`

            let index = Index::load(root_dir?, LoadMode::QuickCheck)?;
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
        Some("ls-models") => {
            let parsed_args = ArgParser::new().optional_flag(&["--name-only", "--stat-only"]).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/ls-models.txt"));
                return Ok(());
            }
            // TODO: impl `--name-only` and `--stat-only`

            let models = Index::list_models(
                &|model| model.name != "dummy",  // filter
                &|model| model,  // no map
                &|model| model.name.to_string(),
            );
            println!("{} models", models.len());

            for model in models.iter() {
                println!("{}", String::from_utf8_lossy(&serde_json::to_vec_pretty(model)?).to_string());
            }
        },
        Some("merge") => {
            let parsed_args = ArgParser::new().args(ArgType::Path, ArgCount::Geq(1)).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/merge.txt"));
                return Ok(());
            }

            let mut index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let bases = parsed_args.get_args();

            for base in bases.iter() {
                index.merge(base)?;
            }

            index.save_to_file()?;
        },
        Some("meta") => {
            let parsed_args = ArgParser::new().flag(&["--get", "--get-all", "--set", "--remove", "--remove-all"]).args(ArgType::String, ArgCount::Geq(0)).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/meta.txt"));
                return Ok(());
            }

            let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let flag = parsed_args.get_flag(0).unwrap();

            match flag.as_str() {
                "--get" => {
                    let key = &parsed_args.get_args_exact(1)?[0];

                    if let Some(value) = index.get_meta_by_key(key.to_string())? {
                        println!("{value}");
                    }
                },
                "--get-all" => {
                    parsed_args.get_args_exact(0)?;
                    let all = index.get_all_meta()?;
                    println!("{}", json::JsonValue::from(all).pretty(4));
                },
                "--set" => {
                    let key_value = parsed_args.get_args_exact(2)?;

                    index.set_meta_by_key(
                        key_value[0].clone(),
                        key_value[1].clone(),
                    )?;
                    println!("metadata set");  // TODO: show change
                },
                "--remove" => {
                    let key = &parsed_args.get_args_exact(1)?[0];

                    index.remove_meta_by_key(key.to_string())?;
                    println!("removed `{key}`");
                },
                "--remove-all" => {
                    parsed_args.get_args_exact(0)?;
                    index.remove_all_meta()?;
                    println!("metadata removed");
                },
                _ => unreachable!(),
            }
        },
        Some("query") => {
            let parsed_args = ArgParser::new().args(ArgType::String, ArgCount::Exact(1)).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/query.txt"));
                return Ok(());
            }

            let index = Index::load(root_dir?, LoadMode::QuickCheck)?;
            let arg = &parsed_args.get_args()[0];

            match arg.as_str() {
                "-i" | "--interactive" => {
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
                query => {
                    let result = single_turn(
                        query,
                        &index,
                    ).await?;
                    println!("{result}");
                },
            }
        },
        Some("remove") | Some("rm") => {
            let parsed_args = ArgParser::new().args(ArgType::Path, ArgCount::Geq(1)).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/remove.txt"));
                return Ok(());
            }

            let mut index = Index::load(root_dir?, LoadMode::QuickCheck)?;
            let files = parsed_args.get_args();

            let remove_count = if files.len() == 1 && files[0] == "--auto" {
                index.remove_auto()?.len()
            }

            else {
                for file in files.iter() {
                    index.remove_file(file.to_string())?;
                }

                files.len()
            };

            index.save_to_file()?;
            println!("removed {remove_count} files from index");
        },
        Some("reset") => {
            let parsed_args = ArgParser::new().flag(&["--soft", "--hard"]).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/reset.txt"));
                return Ok(());
            }

            let soft = parsed_args.get_flag(0).unwrap() == "--soft";

            if soft {
                let mut index = Index::load(root_dir?, LoadMode::OnlyJson)?;
                index.reset_soft()?;
                index.save_to_file()?;
            }

            else {
                Index::reset_hard(&root_dir?)?;
            }
        },
        Some("tfidf") => {
            let parsed_args = ArgParser::new().args(ArgType::String, ArgCount::Geq(1)).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/tfidf.txt"));
                return Ok(());
            }

            let index = Index::load(root_dir?, LoadMode::QuickCheck)?;

            if let Some("--show") = args.get(2).map(|arg| arg.as_str()) {
                let processed_doc = index.get_tfidf_by_chunk_uid(args[3].clone())?;
                println!("{}", processed_doc.render());
                return Ok(());
            }

            let keywords = Keywords::from_raw(parsed_args.get_args());
            let tokenized_keywords = keywords.tokenize();
            let tfidf_results = index.run_tfidf(
                keywords,
                vec![],
            )?;
            let uids = tfidf_results.iter().map(|r| r.id.clone()).collect::<Vec<_>>();
            let chunks = index.get_chunks_by_uid(&uids)?;

            println!("search keywords: {:?}", parsed_args.get_args());
            println!("tokenized keywords: {:?}", tokenized_keywords.iter().map(|(token, _)| token).collect::<Vec<_>>());
            println!("found {} results", chunks.len());

            for (tfidf, chunk) in tfidf_results.iter().zip(chunks.iter()) {
                println!("--------------------------");
                println!("score: {}", tfidf.score);
                println!("file: {}", chunk.render_source());
                println!("title: {}", chunk.title);
                println!("summary: {}", chunk.summary);
            }
        },
        Some("version") => {
            let parsed_args = ArgParser::new().parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/version.txt"));
                return Ok(());
            }

            println!("ragit {}", ragit::VERSION);
        },
        // TODO: suggest similar names
        Some(invalid_command) => {
            println!("{invalid_command:?} is an invalid command.");
        },
        None => {
            println!("Run `rag help` to get help.");
        },
    }

    Ok(())
}

// it starts from "." and goes up until it finds ".ragit"
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
