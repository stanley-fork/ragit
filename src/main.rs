use async_recursion::async_recursion;
use chrono::Local;
use ragit::{
    AddMode,
    Audit,
    Error,
    IIStatus,
    Index,
    INDEX_DIR_NAME,
    Keywords,
    LoadMode,
    MODEL_FILE_NAME,
    MergeMode,
    ProcessedDoc,
    QueryTurn,
    RemoveResult,
    UidQueryConfig,
    get_compatibility_warning,
    merge_and_convert_chunks,
};
use ragit::schema::{ChunkSchema, Prettify};
use ragit_cli::{
    ArgCount,
    ArgParser,
    ArgType,
    get_closest_string,
};
use ragit_fs::{
    basename,
    join,
    join3,
    read_dir,
};
use ragit_pdl::encode_base64;
use serde_json::{Map, Value};
use std::env;
use std::io::Write;

#[tokio::main]
async fn main() {
    let args = env::args().collect::<Vec<_>>();

    match run(args.clone()).await {
        Ok(()) => {},
        Err(e) => {
            // TODO: suggest similar names for some errors
            match e {
                Error::IndexNotFound => {
                    eprintln!("`.ragit/` not found. Make sure that it's a valid ragit repo.");
                },
                Error::InvalidConfigKey(k) => {
                    let similar_key = match find_root() {
                        Ok(root) => match Index::load(root, LoadMode::OnlyJson) {
                            Ok(index) => match index.get_all_configs() {
                                Ok(configs) => get_closest_string(&configs.iter().map(|(key, _)| key.to_string()).collect::<Vec<_>>(), &k),
                                _ => None,
                            },
                            _ => None,
                        },
                        _ => None,
                    };

                    eprintln!(
                        "{k:?} is not a valid key for config.{}",
                        if let Some(similar_key) = similar_key {
                            format!(" There is a similar key: `{similar_key}`.")
                        } else {
                            String::new()
                        },
                    );
                },
                Error::CannotBuild(errors) => {
                    eprintln!("Cannot build knowledge-base due to {} errors", errors.len());

                    for (file, error) in errors.iter() {
                        eprintln!("    {file}: {error}");
                    }
                },
                Error::ApiError(e) => match e {
                    ragit_api::Error::InvalidModelName { name, candidates } => {
                        eprintln!(
                            "{name:?} is not a valid name for a chat model.\n{}",
                            if candidates.is_empty() {
                                if let Ok(root_dir) = find_root() {
                                    if let Ok(index) = Index::load(root_dir, LoadMode::OnlyJson) {
                                        format!(
                                            "Valid model names are: {:?}",
                                            index.models.iter().map(|model| &model.name).collect::<Vec<_>>(),
                                        )
                                    }

                                    else {
                                        String::from("It cannot find any model name. Please make sure that your knowledge-base is not corrupted.")
                                    }
                                }

                                else {
                                    String::from("It cannot find any model name. Please make sure that your knowledge-base is not corrupted.")
                                }
                            } else {
                                format!("Multiple models were matched: {candidates:?}")
                            },
                        );
                    },
                    _ => {
                        eprintln!("{e:?}");
                    },
                },
                Error::CliError { message, span } => {
                    eprintln!("cli error: {message}\n\n{}", ragit_cli::underline_span(
                        &args[..args.len().min(2)].iter().map(|arg| format!("{arg} ")).collect::<Vec<_>>().concat(),
                        &span.0,
                        span.1,
                        span.2,
                    ));
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
            let parsed_args = ArgParser::new()
                .optional_flag(&["--reject", "--force"])
                .optional_flag(&["--all"])
                .optional_flag(&["--dry-run"])
                .short_flag(&["--force"])
                .args(ArgType::Path, ArgCount::Any).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/add.txt"));
                return Ok(());
            }

            let root_dir = root_dir?;
            let mut index = Index::load(root_dir.clone(), LoadMode::QuickCheck)?;
            let add_mode = parsed_args.get_flag(0).map(|flag| AddMode::parse_flag(&flag)).unwrap_or(None);
            let all = parsed_args.get_flag(1).is_some();
            let dry_run = parsed_args.get_flag(2).is_some();
            let ignore_file = index.read_ignore_file()?;

            let mut files = parsed_args.get_args();

            if all {
                if !files.is_empty() {
                    return Err(Error::CliError {
                        message: String::from("You cannot use `--all` options with paths."),
                        span: (String::new(), 0, 0),  // TODO
                    });
                }

                files.push(root_dir.clone());
            }

            else if files.is_empty() {
                return Err(Error::CliError {
                    message: String::from("Please specify which files to add."),
                    span: (String::new(), 0, 0),  // TODO
                });
            }

            // if it's `--reject` mode, it first runs with `--dry-run` mode.
            // if the dry_run has no problem, then it actually runs
            let result = index.add_files(
                &files,
                add_mode,
                dry_run || add_mode == Some(AddMode::Reject),
                &ignore_file,
            )?;

            if add_mode == Some(AddMode::Reject) && !dry_run {
                index.add_files(&files, add_mode, dry_run, &ignore_file)?;
            }

            println!("{result}");
        },
        Some("archive-create") | Some("create-archive") => {
            let parsed_args = ArgParser::new()
                .arg_flag_with_default("--jobs", "4", ArgType::UnsignedInteger)
                .optional_arg_flag("--size-limit", ArgType::UnsignedInteger)
                .arg_flag("--output", ArgType::Path)
                .flag_with_default(&["--no-configs", "--configs"])
                .flag_with_default(&["--no-prompts", "--prompts"])
                .optional_flag(&["--force"])
                .optional_flag(&["--quiet"])
                .short_flag(&["--force", "--output", "--quiet"])
                .parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/archive-create.txt"));
                return Ok(());
            }

            let index = Index::load(root_dir?, LoadMode::QuickCheck)?;
            let jobs = parsed_args.arg_flags.get("--jobs").as_ref().unwrap().parse::<usize>().unwrap();
            let size_limit = parsed_args.arg_flags.get("--size-limit").as_ref().map(|n| n.parse::<u64>().unwrap());
            let output = parsed_args.arg_flags.get("--output").as_ref().unwrap().to_string();
            let include_configs = parsed_args.get_flag(0).unwrap() == "--configs";
            let include_prompts = parsed_args.get_flag(1).unwrap() == "--prompts";
            let force = parsed_args.get_flag(2).is_some();
            let quiet = parsed_args.get_flag(3).is_some();
            index.create_archive(
                jobs,
                size_limit,
                output,
                include_configs,
                include_prompts,
                force,
                quiet,
            )?;
        },
        Some("archive-extract") | Some("extract-archive") => {
            let parsed_args = ArgParser::new()
                .arg_flag_with_default("--jobs", "4", ArgType::UnsignedInteger)
                .arg_flag("--output", ArgType::Path)
                .optional_flag(&["--force"])
                .optional_flag(&["--quiet"])
                .short_flag(&["--force", "--output", "--quiet"])
                .args(ArgType::Path, ArgCount::Geq(1))
                .parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/archive-extract.txt"));
                return Ok(());
            }

            let jobs = parsed_args.arg_flags.get("--jobs").as_ref().unwrap().parse::<usize>().unwrap();
            let output = parsed_args.arg_flags.get("--output").as_ref().unwrap().to_string();
            let archives = parsed_args.get_args();
            let force = parsed_args.get_flag(0).is_some();
            let quiet = parsed_args.get_flag(1).is_some();
            Index::extract_archive(
                &output,
                archives,
                jobs,
                force,
                quiet,
            )?;
        },
        Some("build") => {
            let parsed_args = ArgParser::new()
                .arg_flag_with_default("--jobs", "4", ArgType::UnsignedInteger)
                .optional_flag(&["--quiet"])
                .short_flag(&["--quiet"])
                .parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/build.txt"));
                return Ok(());
            }

            let jobs = parsed_args.arg_flags.get("--jobs").as_ref().unwrap().parse::<usize>().unwrap();
            let quiet = parsed_args.get_flag(0).is_some();
            let mut index = Index::load(root_dir?, LoadMode::QuickCheck)?;
            index.build(jobs, quiet).await?;
        },
        Some("audit") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--this-week"])
                .optional_flag(&["--only-tokens", "--only-costs"])
                .optional_arg_flag("--category", ArgType::String)
                .optional_flag(&["--json"])
                .short_flag(&["--category"])
                .parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/audit.txt"));
                return Ok(());
            }

            let this_week = parsed_args.get_flag(0).is_some();
            let since = (Local::now().timestamp().max(604800) - 604800) as u64;  // a week is 604800 seconds
            let category = parsed_args.arg_flags.get("--category").map(|c| c.to_string());
            let show_tokens = parsed_args.get_flag(1).unwrap_or(String::from("--only-tokens")) != "--only-costs";
            let show_costs = parsed_args.get_flag(1).unwrap_or(String::from("--only-costs")) != "--only-tokens";
            let json_mode = parsed_args.get_flag(2).is_some();
            let index = Index::load(root_dir?, LoadMode::Minimum)?;
            let mut result = index.audit(if this_week { Some(since) } else { None })?;
            let mut total = Audit::default();

            for a in result.values() {
                total += *a;
            }

            result.insert(String::from("total"), total);

            if let Some(category) = &category {
                let result = match result.get(category) {
                    Some(r) => *r,
                    None => {
                        return Err(Error::CliError {
                            message: format!("`{category}` is an invalid category."),
                            span: (String::new(), 0, 0),  // TODO
                        });
                    },
                };

                if json_mode {
                    // This is the worst code I've ever written.
                    println!(
                        "{}{}{}{}{}{}",
                        "{",
                        if show_tokens && show_costs { format!("\"category\": {category:?}, ") } else { String::new() },
                        if show_tokens { format!("\"total tokens\": {}, \"input tokens\": {}, \"output tokens\": {}", result.input_tokens + result.output_tokens, result.input_tokens, result.output_tokens) } else { String::new() },
                        if show_tokens && show_costs { ", " } else { "" },
                        if show_costs { format!("\"total cost\": {:.03}, \"input cost\": {:.03}, \"output cost\": {:.03}", (result.input_cost + result.output_cost) as f64 / 1_000_000_000.0, result.input_cost as f64 / 1_000_000_000.0, result.output_cost as f64 / 1_000_000_000.0) } else { String::new() },
                        "}",
                    );
                }

                else {
                    println!("category: {category}");

                    if show_tokens {
                        println!("    total tokens:  {}", result.input_tokens + result.output_tokens);
                        println!("    input tokens:  {}", result.input_tokens);
                        println!("    output tokens: {}", result.output_tokens);
                    }

                    if show_costs {
                        println!("    total cost:  {:.03}$", (result.input_cost + result.output_cost) as f64 / 1_000_000_000.0);
                        println!("    input cost:  {:.03}$", result.input_cost as f64 / 1_000_000_000.0);
                        println!("    output cost: {:.03}$", result.output_cost as f64 / 1_000_000_000.0);
                    }
                }
            }

            else {
                // for readability, it sorts the keys
                let mut sorted_categories = result.keys().map(|category| category.to_string()).collect::<Vec<_>>();
                sorted_categories.sort();
                sorted_categories = sorted_categories.into_iter().filter(|category| category != "total").collect();
                sorted_categories.insert(0, String::from("total"));

                if json_mode {
                    let mut map = serde_json::Map::new();

                    for category in sorted_categories.iter() {
                        let mut entry = serde_json::Map::new();
                        let audit = result.get(category).unwrap();

                        if show_tokens {
                            entry.insert(String::from("total tokens"), (audit.input_tokens + audit.output_tokens).into());
                            entry.insert(String::from("input tokens"), audit.input_tokens.into());
                            entry.insert(String::from("output tokens"), audit.output_tokens.into());
                        }

                        if show_costs {
                            entry.insert(String::from("total cost"), ((audit.input_cost + audit.output_cost) as f64 / 1_000_000_000.0).into());
                            entry.insert(String::from("input cost"), (audit.input_cost as f64 / 1_000_000_000.0).into());
                            entry.insert(String::from("output cost"), (audit.output_cost as f64 / 1_000_000_000.0).into());
                        }

                        map.insert(category.to_string(), entry.into());
                    }

                    println!("{}", serde_json::to_string_pretty(&map)?);
                }

                else {
                    for category in sorted_categories.iter() {
                        let audit = result.get(category).unwrap();
                        println!("category: {category}");

                        if show_tokens {
                            println!("    total tokens:  {}", audit.input_tokens + audit.output_tokens);
                            println!("    input tokens:  {}", audit.input_tokens);
                            println!("    output tokens: {}", audit.output_tokens);
                        }

                        if show_costs {
                            println!("    total cost:  {:.03}$", (audit.input_cost + audit.output_cost) as f64 / 1_000_000_000.0);
                            println!("    input cost:  {:.03}$", audit.input_cost as f64 / 1_000_000_000.0);
                            println!("    output cost: {:.03}$", audit.output_cost as f64 / 1_000_000_000.0);
                        }
                    }
                }
            }
        },
        Some("cat-file") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--json"])
                .args(ArgType::Query, ArgCount::Exact(1))
                .parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/cat-file.txt"));
                return Ok(());
            }

            let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let query = parsed_args.get_args_exact(1)?.clone();
            let query_result = index.uid_query(&args, UidQueryConfig::new())?;
            let json_mode = parsed_args.get_flag(0).is_some();

            if query_result.has_multiple_matches() {
                return Err(Error::UidQueryError(format!("There're multiple file/chunk that match `{}`. Please give more specific query.", query[0])));
            }

            else if let Some(uid) = query_result.get_chunk_uid() {
                let chunk = index.get_chunk_by_uid(uid)?;

                if json_mode {
                    println!("{:?}", chunk.data);
                }

                else {
                    println!("{}", chunk.data);
                }
            }

            else if let Some((_, uid)) = query_result.get_processed_file() {
                let chunk_uids = index.get_chunks_of_file(uid)?;
                let mut chunks = Vec::with_capacity(chunk_uids.len());

                for chunk_uid in chunk_uids {
                    chunks.push(index.get_chunk_by_uid(chunk_uid)?);
                }

                chunks.sort_by_key(|chunk| chunk.source.sortable_string());
                let chunks = merge_and_convert_chunks(&index, chunks, false /* render_image */)?;

                match chunks.len() {
                    0 => {
                        // empty file
                    },
                    1 => {
                        if json_mode {
                            println!("{:?}", chunks[0].data);
                        }

                        else {
                            println!("{}", chunks[0].data);
                        }
                    },
                    _ => {
                        return Err(Error::BrokenIndex(String::from("Assertion error: `merge_and_convert_chunks` failed to merge chunks of a file. It's likely to be a bug, please open an issue.")));
                    },
                }
            }

            else if let Some(f) = query_result.get_staged_file() {
                return Err(Error::UidQueryError(format!("`{f}` has no chunks yet. Please run `rag build`.")));
            }

            else if let Some(image_uid) = query_result.get_image_uid() {
                let image = index.get_image_schema(image_uid, true)?;

                if json_mode {
                    println!("{:?}", encode_base64(&image.bytes));
                }

                else {
                    std::io::stdout().write_all(&image.bytes)?;
                }
            }

            else {
                return Err(Error::UidQueryError(format!("There's no chunk/file/image that matches `{}`.", query[0])));
            }
        },
        Some("check") => {
            let parsed_args = ArgParser::new().optional_flag(&["--recover"]).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/check.txt"));
                return Ok(());
            }

            let root_dir = root_dir?;
            let recover = parsed_args.get_flag(0).is_some();

            if let Ok(index_version) = Index::check_ragit_version(&root_dir) {
                if recover && get_compatibility_warning(
                    &index_version.to_string(),
                    ragit::VERSION,
                ).is_some() {
                    if let Ok(Some((v1, v2))) = Index::migrate(&root_dir) {
                        println!("migrated from `{v1}` to `{v2}`");
                    }
                }
            }

            match Index::load(root_dir.clone(), LoadMode::OnlyJson) {
                Ok(mut index) => if index.curr_processing_file.is_some() && recover {
                    let recover_result = index.recover()?;
                    index.check()?;
                    println!("recovered from a corrupted knowledge-base: {recover_result}");
                } else {
                    match index.check() {
                        Ok(()) => {
                            println!("everything is fine!");
                        },
                        Err(e) => if recover {
                            let recover_result = index.recover()?;
                            index.check()?;
                            println!("recovered from a corrupted knowledge-base: {recover_result}");
                        } else {
                            return Err(e);
                        }
                    }
                },
                Err(e) => if recover {
                    let mut index = Index::load(root_dir, LoadMode::Minimum)?;
                    let recover_result = index.recover()?;
                    index.check()?;
                    println!("recovered from a corrupted knowledge-base: {recover_result}");
                } else {
                    return Err(e);
                },
            }
        },
        Some("clone") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--quiet"])
                .short_flag(&["--quiet"])
                .args(ArgType::String, ArgCount::Geq(1))
                .parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/clone.txt"));
                return Ok(());
            }

            if root_dir.is_ok() {
                return Err(Error::CannotClone(String::from("You're already inside a knowledge-base. You cannot clone another knowledge-base here.")));
            }

            let args = parsed_args.get_args();
            let quiet = parsed_args.get_flag(0).is_some();
            Index::clone(
                args[0].clone(),
                args.get(1).map(|s| s.to_string()),
                quiet,
            ).await?;
            return Ok(());
        },
        Some("config") => {
            let parsed_args = ArgParser::new().flag(&["--set", "--get", "--get-all"]).args(ArgType::String, ArgCount::Any).parse(&args[2..])?;

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
                    println!("{}", index.get_config_by_key(args[0].clone())?.to_string());
                },
                "--get-all" => {
                    parsed_args.get_args_exact(0)?;  // make sure that there's no dangling args
                    let mut kv = index.get_all_configs()?;
                    kv.sort_by_key(|(k, _)| k.to_string());

                    println!("{}", '{');

                    for (i, (k, v)) in kv.iter().enumerate() {
                        println!(
                            "    {k:?}: {v}{}",
                            if i != kv.len() - 1 { "," } else { "" },
                        );
                    }

                    println!("{}", '}');
                },
                _ => unreachable!(),
            }
        },
        Some("extract-keywords") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--full-schema"])
                .optional_flag(&["--json"])
                .args(ArgType::Query, ArgCount::Exact(1))
                .parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/extract-keywords.txt"));
                return Ok(());
            }

            let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let full_schema = parsed_args.get_flag(0).is_some();
            let json_mode = parsed_args.get_flag(1).is_some();
            let query = &parsed_args.get_args_exact(1)?[0];
            let result = index.extract_keywords(query).await?;

            if full_schema {
                if json_mode {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }

                else {
                    println!("keywords:");
                    println!(
                        "{}",
                        result.keywords.iter().map(
                            |keyword| format!("    {keyword}")
                        ).collect::<Vec<_>>().join("\n"),
                    );
                    println!("extra:");
                    println!(
                        "{}",
                        result.extra.iter().map(
                            |extra| format!("    {extra}")
                        ).collect::<Vec<_>>().join("\n"),
                    );
                }
            } else {
                let mut keywords = result.keywords.clone();

                for e in result.extra.into_iter() {
                    if !keywords.contains(&e) {
                        keywords.push(e);
                    }
                }

                if json_mode {
                    println!("{keywords:?}");
                }

                else {
                    println!("{}", keywords.join("\n"));
                }
            }
        },
        Some("gc") => {
            let parsed_args = ArgParser::new().flag(&["--logs", "--images", "--audit"]).parse(&args[2..])?;

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
                "--images" => {
                    let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
                    let removed = index.gc_images()?;
                    println!("removed {removed} files");
                },
                "--audit" => {
                    let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
                    index.gc_audit()?;
                    println!("removed audit logs");
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
                Some("pipeline") => {
                    println!("{}", include_str!("../docs/config.md"));
                },
                Some("quick-guide") => {
                    println!("{}", include_str!("../docs/quick_guide.md"));
                },
                Some("uid-query") => {
                    println!("{}", include_str!("../docs/uid_query.md"));
                },
                Some(command) => {
                    let mut new_args = args.clone();
                    new_args[1] = command.to_string();
                    new_args[2] = String::from("--help");
                    return run(new_args).await;
                },
                None => {
                    println!("{}", include_str!("../docs/commands/help.txt"));
                },
            }
        },
        Some("ii-build") | Some("build-ii") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--quiet"])
                .short_flag(&["--quiet"])
                .parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/ii-build.txt"));
                return Ok(());
            }

            let mut index = Index::load(root_dir?, LoadMode::QuickCheck)?;
            let quiet = parsed_args.get_flag(0).is_some();
            index.build_ii(quiet)?;
        },
        Some("ii-reset") | Some("reset-ii") => {
            let parsed_args = ArgParser::new().parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/ii-reset.txt"));
                return Ok(());
            }

            let mut index = Index::load(root_dir?, LoadMode::QuickCheck)?;
            index.reset_ii()?;
        },
        Some("ii-status") => {
            let parsed_args = ArgParser::new().parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/ii-status.txt"));
                return Ok(());
            }

            let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let status = match index.ii_status {
                IIStatus::None => "not initialized",
                IIStatus::Complete => "complete",
                IIStatus::Outdated => "outdated",
                IIStatus::Ongoing(_) => "interrupted",
            };
            println!("{status}");
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
            let parsed_args = ArgParser::new()
                .optional_flag(&["--uid-only", "--stat-only"])
                .optional_flag(&["--json"])
                .args(ArgType::Query, ArgCount::Any).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/ls-chunks.txt"));
                return Ok(());
            }

            let uid_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--uid-only";
            let stat_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--stat-only";
            let json_mode = parsed_args.get_flag(1).is_some();
            let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let args = parsed_args.get_args();

            let chunks = if args.is_empty() {
                if !uid_only {
                    if !json_mode {
                        println!("{} chunks", index.chunk_count);
                    }

                    else if stat_only {
                        println!("{}\"chunks\": {}{}", "{", index.chunk_count, "}");
                    }
                }

                if stat_only {
                    return Ok(());
                }

                index.list_chunks(
                    &|_| true,  // no filter
                    &|c| c,  // no map
                    &|chunk: &ChunkSchema| chunk.source.sortable_string(),  // sort by source
                )?
            } else {
                let query = index.uid_query(&args, UidQueryConfig::new().file_or_chunk_only())?;
                let mut chunks = vec![];

                for uid in query.get_chunk_uids() {
                    let chunk = index.get_chunk_by_uid(uid)?;
                    chunks.push(chunk);
                }

                if chunks.is_empty() {
                    for file_uid in query.get_file_uids() {
                        let uids = index.get_chunks_of_file(file_uid)?;

                        for uid in uids.iter() {
                            let chunk = index.get_chunk_by_uid(*uid)?;
                            chunks.push(chunk);
                        }
                    }
                }

                if chunks.is_empty() {
                    return Err(Error::UidQueryError(format!("There's no chunk/file that matches `{}`.", args.join(" "))));
                }

                if !uid_only {
                    if !json_mode {
                        println!("{} chunks", chunks.len());
                    }

                    else if stat_only {
                        println!("{}\"chunks\": {}{}", "{", chunks.len(), "}");
                    }
                }

                if stat_only {
                    return Ok(());
                }

                chunks
            };

            if json_mode {
                if uid_only {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &chunks.iter().map(
                                |chunk| chunk.uid.to_string()
                            ).collect::<Vec<_>>(),
                        )?,
                    );
                }

                else {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&chunks.prettify()?)?,
                    );
                }
            }

            else {
                for chunk in chunks.iter() {
                    if uid_only {
                        println!("{}", chunk.uid);
                        continue;
                    }

                    println!("----------");
                    println!("{}", chunk.render_source());
                    println!("uid: {}", chunk.uid);
                    println!("character_len: {}", chunk.char_len);
                    println!("title: {}", chunk.title);
                    println!("summary: {}", chunk.summary);
                }
            }
        },
        Some("ls-files") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--name-only", "--uid-only", "--stat-only"])
                .optional_flag(&["--json"])
                .args(ArgType::Query, ArgCount::Any).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/ls-files.txt"));
                return Ok(());
            }

            let name_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--name-only";
            let uid_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--uid-only";
            let stat_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--stat-only";
            let json_mode = parsed_args.get_flag(1).is_some();
            let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let args = parsed_args.get_args();

            let files = if args.is_empty() {
                if !uid_only && !name_only {
                    if json_mode && stat_only {
                        println!(
                            "{}\"total files\": {}, \"staged files\": {}, \"processed files\": {}{}",
                            "{",
                            index.staged_files.len() + index.processed_files.len(),
                            index.staged_files.len(),
                            index.processed_files.len(),
                            "}",
                        );
                    }

                    else if !json_mode {
                        println!(
                            "{} total files, {} staged files, {} processed files",
                            index.staged_files.len() + index.processed_files.len(),
                            index.staged_files.len(),
                            index.processed_files.len(),
                        );
                    }
                }

                if stat_only {
                    return Ok(());
                }

                index.list_files(
                    &|_| true,  // no filter
                    &|f| f,  // no map
                    &|f| f.path.to_string(),
                )?
            } else {
                let query = index.uid_query(&args, UidQueryConfig::new().file_only())?;
                let mut files = vec![];
                let mut processed_files_len = 0;
                let mut staged_files_len = 0;

                for (path, uid) in query.get_processed_files() {
                    processed_files_len += 1;
                    files.push(index.get_file_schema(Some(path), Some(uid))?);
                }

                for path in query.get_staged_files() {
                    staged_files_len += 1;
                    files.push(index.get_file_schema(Some(path), None)?);
                }

                if files.is_empty() {
                    return Err(Error::UidQueryError(format!("There's no file that matches `{}`.", args.join(" "))));
                }

                if !uid_only && !name_only {
                    if json_mode && stat_only {
                        println!(
                            "{}\"total files\": {}, \"staged files\": {}, \"processed files\": {}{}",
                            "{",
                            staged_files_len + processed_files_len,
                            staged_files_len,
                            processed_files_len,
                            "}",
                        );
                    }

                    else if !json_mode {
                        println!(
                            "{} total files, {} staged files, {} processed files",
                            staged_files_len + processed_files_len,
                            staged_files_len,
                            processed_files_len,
                        );
                    }
                }

                if stat_only {
                    return Ok(());
                }

                files
            };

            if json_mode {
                if name_only {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &files.iter().map(
                                |file| file.path.to_string()
                            ).collect::<Vec<_>>(),
                        )?,
                    );
                }

                else if uid_only {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &files.iter().map(
                                |file| file.uid.to_string()
                            ).collect::<Vec<_>>(),
                        )?,
                    );
                }

                else {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&files.prettify()?)?,
                    );
                }
            }

            else {
                for file in files.iter() {
                    if name_only {
                        println!("{}", file.path);
                        continue;
                    }

                    else if uid_only {
                        println!("{}", file.uid);
                        continue;
                    }

                    println!("--------");
                    println!("name: {}{}", file.path, if file.is_processed { String::new() } else { String::from(" (not processed yet)") });

                    if file.is_processed {
                        println!("length: {}", file.length);
                        println!("uid: {}", file.uid);
                        println!("chunks: {}", file.chunks);
                    }
                }
            }
        },
        Some("ls-images") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--uid-only", "--stat-only"])
                .optional_flag(&["--json"])
                .args(ArgType::Query, ArgCount::Any).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/ls-images.txt"));
                return Ok(());
            }

            let uid_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--uid-only";
            let stat_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--stat-only";
            let json_mode = parsed_args.get_flag(1).is_some();
            let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let args = parsed_args.get_args();

            let images = if args.is_empty() {
                let result = index.list_images(
                    &|_| true,  // no filter
                    &|image| image,  // no map
                    &|_| 0,  // no sort
                )?;

                result
            } else {
                let query = index.uid_query(&args, UidQueryConfig::new())?;
                let mut image_uids = vec![];

                for (_, uid) in query.get_processed_files() {
                    for image_uid in index.get_images_of_file(uid)? {
                        image_uids.push(image_uid);
                    }
                }

                for uid in query.get_chunk_uids() {
                    let chunk = index.get_chunk_by_uid(uid)?;

                    for image_uid in chunk.images {
                        image_uids.push(image_uid);
                    }
                }

                for image_uid in query.get_image_uids() {
                    image_uids.push(image_uid);
                }

                if image_uids.is_empty() {
                    return Err(Error::UidQueryError(format!("There's no chunk/file/image that matches `{}`.", args.join(" "))));
                }

                let mut result = Vec::with_capacity(image_uids.len());

                for image_uid in image_uids.iter() {
                    result.push(index.get_image_schema(*image_uid, false)?);
                }

                result
            };

            if uid_only {
                if json_mode {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &images.iter().map(
                                |image| image.uid.to_string()
                            ).collect::<Vec<_>>(),
                        )?,
                    );
                }

                else {
                    for image in images.iter() {
                        println!("{}", image.uid);
                    }
                }
            }

            else if stat_only {
                if json_mode {
                    println!("{}\"images\": {}{}", '{', images.len(), '}');
                }

                else {
                    println!("{} images", images.len());
                }
            }

            else {
                if json_mode {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&images.prettify()?)?,
                    );
                }

                else {
                    println!("{} images", images.len());

                    for image in images.iter() {
                        println!("--------");
                        println!("uid: {}", image.uid);
                        println!("explanation: {}", image.explanation);
                        println!("extracted_text: {}", image.extracted_text);
                        println!("size: {}", image.size);
                    }
                }
            }
        },
        Some("ls-models") => {
            let parsed_args = ArgParser::new().optional_flag(&["--name-only", "--stat-only"]).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/ls-models.txt"));
                return Ok(());
            }

            let name_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--name-only";
            let stat_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--stat-only";
            let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let models = Index::list_models(
                &join3(
                    &index.root_dir,
                    INDEX_DIR_NAME,
                    MODEL_FILE_NAME,
                )?,
                &|model| model.name != "dummy",  // filter
                &|model| model,  // no map
                &|model| model.name.to_string(),
            )?;
            println!("{} models", models.len());

            if stat_only {
                return Ok(());
            }

            for model in models.iter() {
                if name_only {
                    println!("{}", model.name);
                    continue;
                }

                println!("--------");
                println!("name: {}", model.name);
                println!("api_provider: {}", model.api_provider);

                if let Some(api_env_var) = &model.api_env_var {
                    println!("api_key_env_var: {api_env_var}");
                }

                println!("can_read_images: {}", model.can_read_images);
                println!("dollars_per_1b_input_tokens: {}", model.dollars_per_1b_input_tokens);
                println!("dollars_per_1b_output_tokens: {}", model.dollars_per_1b_output_tokens);
            }
        },
        Some("ls-terms") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--term-only", "--stat-only"])
                .optional_flag(&["--json"])
                .args(ArgType::Query, ArgCount::Any).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/ls-terms.txt"));
                return Ok(());
            }
            let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let term_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--term-only";
            let stat_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--stat-only";
            let json_mode = parsed_args.get_flag(1).is_some();
            let args = parsed_args.get_args();

            let processed_doc = if args.is_empty() {
                let mut result = ProcessedDoc::empty();

                for chunk_uid in index.get_all_chunk_uids()? {
                    result.extend(&index.get_tfidf_by_chunk_uid(chunk_uid)?);
                }

                result
            } else {
                let query = index.uid_query(&args, UidQueryConfig::new().file_or_chunk_only().no_staged_file())?;

                if query.has_multiple_matches() {
                    return Err(Error::UidQueryError(format!("There're {} chunks/files that match `{}`. Please give more specific query.", query.len(), args.join(" "))));
                }

                else if query.is_empty() {
                    return Err(Error::UidQueryError(format!("There's no chunk or file that matches `{}`.", args.join(" "))));
                }

                else if let Some((_, uid)) = query.get_processed_file() {
                    index.get_tfidf_by_file_uid(uid)?
                }

                else if let Some(uid) = query.get_chunk_uid() {
                    index.get_tfidf_by_chunk_uid(uid)?
                }

                else {
                    unreachable!()
                }
            };

            println!("{}", processed_doc.render(term_only, stat_only, json_mode));
            return Ok(());
        },
        Some("merge") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--ignore", "--force", "--interactive", "--reject"])
                .optional_flag(&["--dry-run"])
                .optional_arg_flag("--prefix", ArgType::Path)
                .optional_flag(&["--quiet"])
                .short_flag(&["--force", "--quiet"])
                .args(ArgType::Path, ArgCount::Geq(1)).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/merge.txt"));
                return Ok(());
            }

            let mut index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let bases = parsed_args.get_args();
            let merge_mode = MergeMode::parse_flag(&parsed_args.get_flag(0).unwrap_or(String::from("--ignore"))).unwrap();
            let dry_run = parsed_args.get_flag(1).is_some();
            let quiet = parsed_args.get_flag(2).is_some();

            // if it's `--reject` mode, it first runs with `--dry-run` mode.
            // if the dry_run has no problem, then it actually runs
            for base in bases.iter() {
                index.merge(
                    base.to_string(),
                    parsed_args.arg_flags.get("--prefix").map(|p| p.to_string()),
                    merge_mode,

                    // if it's run twice, the first run has to be `--quiet`.
                    merge_mode == MergeMode::Reject && !dry_run || quiet,
                    dry_run || merge_mode == MergeMode::Reject,
                )?;
            }

            if merge_mode == MergeMode::Reject && !dry_run {
                for base in bases.iter() {
                    index.merge(
                        base.to_string(),
                        parsed_args.arg_flags.get("--prefix").map(|p| p.to_string()),
                        merge_mode,
                        quiet,
                        dry_run,
                    )?;
                }
            }
        },
        Some("meta") => {
            let parsed_args = ArgParser::new()
                .flag(&["--get", "--get-all", "--set", "--remove", "--unset", "--remove-all", "--unset-all"])
                .optional_flag(&["--json"])
                .args(ArgType::String, ArgCount::Any).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/meta.txt"));
                return Ok(());
            }

            let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let flag = parsed_args.get_flag(0).unwrap();
            let json_mode = parsed_args.get_flag(1).is_some();

            match flag.as_str() {
                "--get" => {
                    let key = &parsed_args.get_args_exact(1)?[0];

                    if let Some(value) = index.get_meta_by_key(key.to_string())? {
                        if json_mode {
                            println!("{value:?}");
                        }

                        else {
                            println!("{value}");
                        }
                    }

                    else {
                        return Err(Error::NoSuchMeta(key.to_string()));
                    }
                },
                "--get-all" => {
                    parsed_args.get_args_exact(0)?;
                    let all = index.get_all_meta()?;
                    println!("{}", serde_json::to_string_pretty(&all)?);
                },
                "--set" => {
                    let key_value = parsed_args.get_args_exact(2)?;
                    let (key, value) = (
                        key_value[0].to_string(),
                        key_value[1].to_string(),
                    );
                    let prev_value = index.get_meta_by_key(key.clone())?;
                    index.set_meta_by_key(
                        key.clone(),
                        value.clone(),
                    )?;
                    let new_value = index.get_meta_by_key(key.clone())?.unwrap();

                    if let Some(prev_value) = prev_value {
                        println!("metadata set `{key}`: `{prev_value}` -> `{new_value}`", );
                    }

                    else {
                        println!("metadata set `{key}`: `{new_value}`");
                    }
                },
                "--remove" | "--unset" => {
                    let key = &parsed_args.get_args_exact(1)?[0];
                    let prev_value = index.remove_meta_by_key(key.to_string())?;

                    println!("metadata unset `{key}`: `{prev_value}`");
                },
                "--remove-all" | "--unset-all" => {
                    parsed_args.get_args_exact(0)?;
                    index.remove_all_meta()?;
                    println!("metadata removed");
                },
                _ => unreachable!(),
            }
        },
        Some("migrate") => {
            let parsed_args = ArgParser::new().parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/migrate.txt"));
                return Ok(());
            }

            let root_dir = root_dir?;

            if let Some((v1, v2)) = Index::migrate(&root_dir)? {
                println!("migrated from `{v1}` to `{v2}`");
            }

            let mut index = Index::load(root_dir, LoadMode::Minimum)?;
            let recover_result = index.recover()?;

            if !recover_result.is_empty() {
                println!("recovered from a corrupted knowledge-base: {recover_result}");
            }
        },
        Some("push") => {
            let parsed_args = ArgParser::new()
                .optional_arg_flag("--remote", ArgType::Path)
                .flag_with_default(&["--no-configs", "--configs"])
                .flag_with_default(&["--no-prompts", "--prompts"])
                .optional_flag(&["--quiet"])
                .short_flag(&["--quiet"])
                .args(ArgType::String, ArgCount::None)
                .parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/push.txt"));
                return Ok(());
            }

            let index = Index::load(root_dir?, LoadMode::QuickCheck)?;
            let remote = parsed_args.arg_flags.get("--remote").map(|s| s.to_string());
            let include_configs = parsed_args.get_flag(0).unwrap() == "--configs";
            let include_prompts = parsed_args.get_flag(1).unwrap() == "--prompts";
            let quiet = parsed_args.get_flag(2).is_some();
            index.push(
                remote,
                include_configs,
                include_prompts,
                quiet,
            ).await?;
        },
        Some("query") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--interactive", "--multi-turn"])
                .optional_flag(&["--json"])
                .optional_flag(&["--enable-ii", "--disable-ii"])
                .optional_flag(&["--enable-rag", "--disable-rag"])
                .optional_flag(&["--super-rerank", "--no-super-rerank"])
                .optional_arg_flag("--max-summaries", ArgType::UnsignedInteger)
                .optional_arg_flag("--max-retrieval", ArgType::UnsignedInteger)
                .short_flag(&["--interactive"])
                .args(ArgType::String, ArgCount::Any).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/query.txt"));
                return Ok(());
            }

            let mut index = Index::load(root_dir?, LoadMode::OnlyJson)?;

            if let Some(enable_ii) = parsed_args.get_flag(2) {
                index.query_config.enable_ii = enable_ii == "--enable-ii";
            }

            if let Some(enable_rag) = parsed_args.get_flag(3) {
                index.query_config.enable_rag = enable_rag == "--enable-rag";
            }

            if let Some(super_rerank) = parsed_args.get_flag(4) {
                index.query_config.super_rerank = super_rerank == "--super-rerank";
            }

            if let Some(max_summaries) = parsed_args.arg_flags.get("--max-summaries") {
                index.query_config.max_summaries = max_summaries.parse().unwrap();
            }

            if let Some(max_retrieval) = parsed_args.arg_flags.get("--max-retrieval") {
                index.query_config.max_retrieval = max_retrieval.parse().unwrap();
            }

            let interactive_mode = parsed_args.get_flag(0).is_some();
            let json_mode = parsed_args.get_flag(1).is_some();

            match (interactive_mode, json_mode) {
                (true, true) => {
                    return Err(Error::CliError {
                        message: String::from("You cannot query interactively in a json mode."),
                        span: (String::new(), 0, 0),  // TODO
                    });
                },
                (true, _) => {
                    let mut history = vec![];

                    loop {
                        let mut curr_input = String::new();
                        print!(">>> ");
                        std::io::stdout().flush()?;
                        std::io::stdin().read_line(&mut curr_input)?;
                        let response = index.query(
                            &curr_input,
                            history.clone(),
                        ).await?;
                        println!("{}", response.response);
                        history.push(QueryTurn::new(curr_input, response));
                    }
                },
                _ => {
                    let response = index.query(
                        &parsed_args.get_args_exact(1)?[0],
                        vec![],  // no history
                    ).await?;

                    if json_mode {
                        println!("{}", serde_json::to_string_pretty(&response.prettify()?)?);
                    }

                    else {
                        println!("{}", response.response);

                        if !response.retrieved_chunks.is_empty() {
                            println!("\n---- sources ----");

                            for chunk in response.retrieved_chunks.iter() {
                                println!("{} ({})", chunk.render_source(), chunk.uid.get_short_name());
                            }
                        }
                    }
                },
            }
        },
        Some("remove") | Some("rm") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--dry-run"])
                .optional_flag(&["--recursive"])
                .optional_flag(&["--auto"])
                .optional_flag(&["--all"])
                .optional_flag(&["--staged", "--processed"])
                .short_flag(&["--recursive"])
                .args(ArgType::Path, ArgCount::Any)
                .parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/remove.txt"));
                return Ok(());
            }

            let root_dir = root_dir?;
            let mut index = Index::load(root_dir.clone(), LoadMode::QuickCheck)?;
            let dry_run = parsed_args.get_flag(0).is_some();
            let mut recursive = parsed_args.get_flag(1).is_some();
            let auto = parsed_args.get_flag(2).is_some();
            let all = parsed_args.get_flag(3).is_some();
            let staged = parsed_args.get_flag(4).is_none() || parsed_args.get_flag(4) == Some(String::from("--staged"));
            let processed = parsed_args.get_flag(4).is_none() || parsed_args.get_flag(4) == Some(String::from("--processed"));
            let mut files = parsed_args.get_args();
            let mut result = RemoveResult::default();

            if all {
                if !files.is_empty() {
                    return Err(Error::CliError {
                        message: String::from("You cannot use `--all` options with paths."),
                        span: (String::new(), 0, 0),  // TODO
                    });
                }

                if index.staged_files.is_empty() && index.processed_files.is_empty() {
                    println!("removed {} staged files and {} processed files", result.staged, result.processed);
                    return Ok(());
                }

                files = vec![root_dir];
                recursive = true;
            }

            else if files.is_empty() {
                return Err(Error::CliError {
                    message: String::from("Please specify which files to remove."),
                    span: (String::new(), 0, 0),  // TODO
                });
            }

            for file in files.iter() {
                result += index.remove_file(
                    file.to_string(),
                    true,  // dry_run
                    recursive,
                    auto,
                    staged,
                    processed,
                )?;
            }

            if result.is_empty() {
                return Err(Error::NoFileToRemove);
            }

            if !dry_run {
                for file in files.iter() {
                    index.remove_file(
                        file.to_string(),
                        dry_run,
                        recursive,
                        auto,
                        staged,
                        processed,
                    )?;
                }
            }

            println!("removed {} staged files and {} processed files", result.staged, result.processed);
        },
        Some("retrieve-chunks") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--uid-only"])
                .optional_flag(&["--json"])
                .flag_with_default(&["--rerank", "--no-rerank"])
                .optional_arg_flag("--max-retrieval", ArgType::UnsignedInteger)
                .optional_arg_flag("--max-summaries", ArgType::UnsignedInteger)
                .args(ArgType::Query, ArgCount::Exact(1)).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/retrieve-chunks.txt"));
                return Ok(());
            }

            let uid_only = parsed_args.get_flag(0).is_some();
            let json_mode = parsed_args.get_flag(1).is_some();
            let rerank = parsed_args.get_flag(2).unwrap() == "--rerank";
            let index = Index::load(root_dir?, LoadMode::OnlyJson)?;

            let max_retrieval = match parsed_args.arg_flags.get("--max-chunks") {
                Some(n) => n.parse::<usize>().unwrap(),
                None => index.query_config.max_retrieval,
            };
            let max_summaries = match parsed_args.arg_flags.get("--max-summaries") {
                Some(n) => n.parse::<usize>().unwrap(),
                None => index.query_config.max_summaries,
            };
            let query = parsed_args.get_args_exact(1)?[0].clone();

            let keywords = {
                let keywords = index.extract_keywords(&parsed_args.get_args_exact(1)?[0]).await?;

                if keywords.is_empty() {
                    eprintln!("Warning: failed to extract keywords!");
                    Keywords::from_raw(parsed_args.get_args())
                }

                else {
                    keywords
                }
            };
            let tfidf_results = index.run_tfidf(
                keywords,
                max_summaries,
            )?;
            let mut chunks = Vec::with_capacity(tfidf_results.len());

            for tfidf_result in tfidf_results.iter() {
                chunks.push(index.get_chunk_by_uid(tfidf_result.id)?);
            }

            if rerank {
                chunks = index.summaries_to_chunks(
                    &query,
                    chunks,
                    max_retrieval,
                ).await?;
            }

            if json_mode {
                if uid_only {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &chunks.iter().map(
                                |chunk| chunk.uid.to_string()
                            ).collect::<Vec<_>>(),
                        )?,
                    );
                }

                else {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &chunks.iter().map(
                                |chunk| [
                                    (String::from("uid"), chunk.uid.to_string().into()),
                                    (String::from("source"), chunk.render_source().into()),
                                    (String::from("title"), chunk.title.to_string().into()),
                                    (String::from("summary"), chunk.summary.to_string().into()),
                                ].into_iter().collect::<Map<String, Value>>(),
                            ).collect::<Vec<_>>(),
                        )?,
                    );
                }
            }

            else {
                for chunk in chunks.iter() {
                    if uid_only {
                        println!("{}", chunk.uid);
                        continue;
                    }

                    println!("--------------------------");
                    println!("uid: {}", chunk.uid);
                    println!("source: {}", chunk.render_source());
                    println!("title: {}", chunk.title);
                    println!("summary: {}", chunk.summary);
                }
            }
        },
        Some("status") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--json"])
                .args(ArgType::Query, ArgCount::None).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/status.txt"));
                return Ok(());
            }

            let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let json_mode = parsed_args.get_flag(0).is_some();

            if json_mode {
                let mut result = Map::new();
                result.insert(String::from("staged files"), index.staged_files.clone().into());
                result.insert(String::from("processed files"), index.processed_files.keys().map(|key| key.to_string()).collect::<Vec<_>>().into());
                result.insert(String::from("chunks"), index.chunk_count.into());
                result.insert(
                    String::from("inverted index"),
                    match index.ii_status {
                        IIStatus::None => "none",
                        IIStatus::Complete => "clean",
                        IIStatus::Outdated
                        | IIStatus::Ongoing(_) => "dirty",
                    }.into(),
                );
                result.insert(
                    String::from("build status"),
                    if index.curr_processing_file.is_some() { "dirty" } else { "clean" }.into(),
                );

                println!("{}", serde_json::to_string_pretty(&result)?);
            }

            else {
                // TODO: Do I have to sort this? If I'm to sort this, why not just call `index.staged_files.sort()` after `rag add`?
                let staged_files = index.staged_files[0..(index.staged_files.len().min(5))].to_vec();
                let processed_files = index.processed_files.keys().take(5).map(|f| f.to_string()).collect::<Vec<_>>();

                if staged_files.len() > 0 {
                    println!("The knowledge-base is not complete yet. Run `rag build` to finish building the knowledge-base.");
                } else if index.curr_processing_file.is_some() {
                    println!("Build status is dirty. Run `rag check --recover` to clean up the knowledge-base. It may take a while.");
                } else if index.ii_status == IIStatus::None {
                    println!("Inverted index not found. Run `rag ii-build` to speed up chunk retrievals.");
                } else if index.ii_status != IIStatus::Complete {
                    println!("Inverted index is dirty. Run `rag ii-build` to clean up and build an inverted index.");
                } else {
                    println!("The knowledge-base is clean. You can run `rag query`.");
                }

                // TODO: how about counting images?
                println!("");
                println!("chunks: {}", index.chunk_count);
                println!("processed files: {}", index.processed_files.len());

                for file in processed_files.iter() {
                    println!("    {file}");
                }

                if index.processed_files.len() > processed_files.len() {
                    let d = index.processed_files.len() - processed_files.len();
                    println!("    ... ({d} more file{})", if d == 1 { "" } else { "s" });
                }

                println!("");
                println!("staged files: {}", index.staged_files.len());

                for file in staged_files.iter() {
                    println!("    {file}");
                }

                if index.staged_files.len() > staged_files.len() {
                    let d = index.staged_files.len() - staged_files.len();
                    println!("    ... ({d} more file{})", if d == 1 { "" } else { "s" });
                }

                println!("");
                println!(
                    "inverted index: {}",
                    match index.ii_status {
                        IIStatus::None => "none",
                        IIStatus::Complete => "clean",
                        IIStatus::Outdated
                        | IIStatus::Ongoing(_) => "dirty",
                    },
                );
                println!(
                    "build status: {}",
                    if index.curr_processing_file.is_some() { "dirty" } else { "clean" },
                );
            }
        },
        // tmp command for testing `Index::summary_file`
        // this interface is likely to change
        Some("summary-file") => {
            let mut index = Index::load(root_dir?, LoadMode::QuickCheck)?;

            for file_uid in index.get_all_file_uids() {
                index.summary_file(file_uid).await?;
            }
        },
        Some("tfidf") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--uid-only"])
                .optional_flag(&["--json"])
                .flag_with_default(&["--keyword", "--query"])
                .arg_flag_with_default("--limit", "10", ArgType::UnsignedInteger)
                .args(ArgType::Query, ArgCount::Exact(1)).parse(&args[2..])?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/tfidf.txt"));
                return Ok(());
            }

            let uid_only = parsed_args.get_flag(0).is_some();
            let json_mode = parsed_args.get_flag(1).is_some();
            let query_mode = parsed_args.get_flag(2).unwrap_or(String::new()) == "--query";

            let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let started_at = std::time::Instant::now();
            let keywords = if query_mode {
                let keywords = index.extract_keywords(&parsed_args.get_args_exact(1)?[0]).await?;

                if keywords.is_empty() {
                    eprintln!("Warning: failed to extract keywords!");
                    Keywords::from_raw(parsed_args.get_args())
                }

                else {
                    keywords
                }
            } else {
                Keywords::from_raw(parsed_args.get_args())
            };
            let tokenized_keywords = keywords.tokenize();
            let limit = parsed_args.arg_flags.get("--limit").map(|n| n.parse::<usize>().unwrap()).unwrap();

            if !uid_only && !json_mode {
                println!("search keywords: {:?}", parsed_args.get_args());
                println!("tokenized keywords: {:?}", tokenized_keywords.iter().map(|(token, _)| token).collect::<Vec<_>>());

                match index.ii_status {
                    IIStatus::None => if index.query_config.enable_ii {
                        println!("inverted-index not found");
                    } else {
                        println!("inverted-index disabled");
                    },
                    IIStatus::Complete => if index.query_config.enable_ii {
                        println!("inverted-index found");
                    } else {
                        println!("inverted-index found, but is disabled");
                    },
                    IIStatus::Ongoing(_)
                    | IIStatus::Outdated => if index.query_config.enable_ii {
                        println!("inverted-index is corrupted. You may `rag ii-build` to build it from scratch.");
                    } else {
                        println!("inverted-index is corrupted, but not enabled anyway.");
                    },
                }
            }

            let tfidf_results = index.run_tfidf(
                keywords,
                limit,
            )?;
            let mut chunks = Vec::with_capacity(tfidf_results.len());

            for tfidf_result in tfidf_results.iter() {
                let uid = tfidf_result.id;
                chunks.push(index.get_chunk_by_uid(uid)?);
            }

            if !uid_only && !json_mode {
                println!("found {} results", chunks.len());
            }

            if json_mode {
                if uid_only {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &chunks.iter().map(
                                |chunk| chunk.uid.to_string()
                            ).collect::<Vec<_>>(),
                        )?,
                    );
                }

                else {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &tfidf_results.iter().zip(chunks.iter()).map(
                                |(tfidf, chunk)| [
                                    (String::from("score"), Value::from(tfidf.score)),
                                    (String::from("uid"), chunk.uid.to_string().into()),
                                    (String::from("source"), chunk.render_source().into()),
                                    (String::from("title"), chunk.title.to_string().into()),
                                    (String::from("summary"), chunk.summary.to_string().into()),
                                ].into_iter().collect::<Map<String, Value>>(),
                            ).collect::<Vec<_>>(),
                        )?,
                    );
                }
            }

            else {
                for (tfidf, chunk) in tfidf_results.iter().zip(chunks.iter()) {
                    if uid_only {
                        println!("{}", chunk.uid);
                        continue;
                    }

                    println!("--------------------------");
                    println!("score: {}", tfidf.score);
                    println!("uid: {}", chunk.uid);
                    println!("source: {}", chunk.render_source());
                    println!("title: {}", chunk.title);
                    println!("summary: {}", chunk.summary);
                }
            }

            let ms_took = std::time::Instant::now().duration_since(started_at).as_millis();

            if !uid_only && !json_mode {
                if ms_took > 9999 {
                    println!("took {} seconds", ms_took / 1000);
                }

                else {
                    println!("took {ms_took} ms");
                }
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
        Some(invalid_command) => {
            // TODO: this is a very bad idea. I need a way to programatically load the list
            // of the commands.
            let similar_command = get_closest_string(
                &[
                    "add",
                    "archive-create", "create-archive",
                    "archive-extract", "extract-archive",
                    "build",
                    "audit",
                    "cat-file",
                    "check",
                    "clone",
                    "config",
                    "extract-keywords",
                    "gc",
                    "help",
                    "ii-build", "build-ii",
                    "ii-reset", "reset-ii",
                    "ii-status",
                    "init",
                    "ls-chunks",
                    "ls-files",
                    "ls-images",
                    "ls-models",
                    "ls-terms",
                    "merge",
                    "meta",
                    "migrate",
                    "push",
                    "query",
                    "remove", "rm",
                    "retrieve-chunks",
                    "status",
                    "tfidf",
                    "version",
                ].iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                invalid_command,
            );

            return Err(Error::CliError {
                message: format!(
                    "`{invalid_command}` is an invalid command. {}",
                    if let Some(similar_command) = similar_command {
                        format!("There is a similar command: `{similar_command}`.")
                    } else {
                        String::from("Run `rag help` to get help.")
                    },
                ),
                span: (String::new(), 0, 0),  // TODO
            });
        },
        None => {
            return Err(Error::CliError {
                message: String::from("Run `rag help` to get help."),
                span: (String::new(), 0, 0),  // TODO
            });
        },
    }

    Ok(())
}

// it starts from "." and goes up until it finds ".ragit"
// you can run git commands anywhere inside a repo, and I want ragit to be like that
fn find_root() -> Result<String, Error> {
    let mut curr = String::from(".");

    // FIXME: why is it allocating Vec twice?
    loop {
        let curr_files_ = read_dir(&curr, false)?;
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
