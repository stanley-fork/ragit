use async_recursion::async_recursion;
use chrono::{DateTime, Days, Local};
use ragit::{
    AddMode,
    AgentAction,
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
    PullResult,
    PushResult,
    QueryTurn,
    RemoveResult,
    SummaryMode,
    UidOrStagedFile,
    UidQueryConfig,
    get_build_options,
    get_compatibility_warning,
    into_multi_modal_contents,
    render_query_turns,
};
use ragit::schema::{ChunkSchema, Prettify};
use ragit_api::{Model, ModelRaw, get_model_by_name};
use ragit_cli::{
    ArgCount,
    ArgParser,
    ArgType,
    Span,
    get_closest_string,
    parse_pre_args,
};
use ragit_fs::{
    basename,
    create_dir,
    exists,
    join,
    join3,
    read_dir,
    read_string,
    set_current_dir,
};
use ragit_pdl::{
    Pdl,
    encode_base64,
    parse_schema,
    render_pdl_schema,
};
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::env;
use std::io::Write;

#[tokio::main]
async fn main() {
    let args = env::args().collect::<Vec<_>>();

    match run(args.clone()).await {
        Ok(()) => {},
        Err(e) => {
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
                Error::FeatureNotEnabled { action, feature } => {
                    eprintln!("In order to {action}, you have to enable feature {feature}.");
                },
                Error::InvalidModelName { name, candidates } => {
                    if candidates.is_empty() {
                        let all_model_names = if let Ok(root_dir) = find_root() {
                            if let Ok(index) = Index::load(root_dir, LoadMode::OnlyJson) {
                                index.models.iter().map(|model| model.name.to_string()).collect::<Vec<_>>()
                            }

                            else {
                                vec![]
                            }
                        }

                        else {
                            vec![]
                        };

                        eprintln!("No model matches `{name}`. Valid model names are: {}", all_model_names.join(", "));
                    }

                    else {
                        eprintln!("There are multiple models that match `{name}`: {}", candidates.join(", "));
                    }
                },
                Error::DeprecatedConfig { key, message } => {
                    eprintln!("Config `{key}` is deprecated!\n{message}");
                },
                Error::CliError { message, span } => {
                    eprintln!("cli error: {message}{}",
                        if let Some(span) = span {
                            format!("\n\n{}", ragit_cli::underline_span(&span))
                        } else {
                            String::new()
                        },
                    );
                },
                Error::DirtyKnowledgeBase => {
                    eprintln!("The knowledge-base is dirty. Run `rag check --recover`.");
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
    // TODO: `-C` option is not documented
    // it parses `rag [-C <path>] <command> <args>`.
    // `-C <path>` is in `pre_args` and the remainings are in `args`.
    let (args, pre_args) = parse_pre_args(&args)?;

    if let Some(path) = pre_args.arg_flags.get("-C") {
        set_current_dir(path)?;
    }

    let root_dir = find_root().map_err(|_| Error::IndexNotFound);

    match args.get(1).map(|arg| arg.as_str()) {
        Some("add") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--reject", "--force"])
                .optional_flag(&["--all"])
                .optional_flag(&["--dry-run"])
                .short_flag(&["--force"])
                .args(ArgType::String, ArgCount::Any)  // paths
                .parse(&args, 2)?;

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
                        message: String::from("You cannot use `--all` option with paths."),
                        span: None,
                    });
                }

                files.push(root_dir.clone());
            }

            else if files.is_empty() {
                return Err(Error::CliError {
                    message: String::from("Please specify which files to add."),
                    span: Span::End.render(&args, 2),
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
        Some("archive-create") | Some("create-archive") | Some("archive") => {
            let parsed_args = ArgParser::new()
                .arg_flag_with_default("--jobs", "4", ArgType::uinteger())
                .optional_arg_flag("--size-limit", ArgType::file_size_between(Some(4096), None))
                .arg_flag("--output", ArgType::String)
                .flag_with_default(&["--no-configs", "--configs"])
                .flag_with_default(&["--no-prompts", "--prompts"])
                .flag_with_default(&["--no-queries", "--queries"])
                .optional_flag(&["--force"])
                .optional_flag(&["--quiet"])
                .short_flag(&["--force", "--output", "--quiet"])
                .parse(&args, 2)?;

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
            let include_queries = parsed_args.get_flag(2).unwrap() == "--queries";
            let force = parsed_args.get_flag(3).is_some();
            let quiet = parsed_args.get_flag(4).is_some();
            index.create_archive(
                jobs,
                size_limit,
                output,
                include_configs,
                include_prompts,
                include_queries,
                force,
                quiet,
            )?;
        },
        Some("archive-extract") | Some("extract-archive") | Some("extract") => {
            let parsed_args = ArgParser::new()
                .arg_flag_with_default("--jobs", "4", ArgType::uinteger())
                .arg_flag("--output", ArgType::String)
                .optional_flag(&["--force"])
                .optional_flag(&["--quiet"])
                .flag_with_default(&["--ii", "--no-ii"])
                .short_flag(&["--force", "--output", "--quiet"])
                .args(ArgType::String, ArgCount::Geq(1))
                .parse(&args, 2)?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/archive-extract.txt"));
                return Ok(());
            }

            let jobs = parsed_args.arg_flags.get("--jobs").as_ref().unwrap().parse::<usize>().unwrap();
            let output = parsed_args.arg_flags.get("--output").as_ref().unwrap().to_string();
            let archives = parsed_args.get_args();
            let force = parsed_args.get_flag(0).is_some();
            let quiet = parsed_args.get_flag(1).is_some();
            let ii = parsed_args.get_flag(2).unwrap() == "--ii";
            Index::extract_archive(
                &output,
                archives,
                jobs,
                force,
                ii,
                quiet,
            )?;
        },
        Some("build") => {
            let parsed_args = ArgParser::new()
                .arg_flag_with_default("--jobs", "8", ArgType::uinteger())
                .optional_flag(&["--quiet"])
                .short_flag(&["--quiet"])
                .parse(&args, 2)?;

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
                .optional_arg_flag("--category", ArgType::String)  // NOTE: there's no `ArgType` for audit categories
                .optional_flag(&["--json"])
                .short_flag(&["--category", "--json"])
                .parse(&args, 2)?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/audit.txt"));
                return Ok(());
            }

            let this_week = parsed_args.get_flag(0).is_some();
            let since = Local::now().checked_sub_days(Days::new(7)).unwrap();
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
                            span: None,
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
                        if show_costs { format!("\"total cost\": {:.03}, \"input cost\": {:.03}, \"output cost\": {:.03}", (result.input_cost + result.output_cost) as f64 / 1_000_000.0, result.input_cost as f64 / 1_000_000.0, result.output_cost as f64 / 1_000_000.0) } else { String::new() },
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
                        println!("    total cost:  {:.03}$", (result.input_cost + result.output_cost) as f64 / 1_000_000.0);
                        println!("    input cost:  {:.03}$", result.input_cost as f64 / 1_000_000.0);
                        println!("    output cost: {:.03}$", result.output_cost as f64 / 1_000_000.0);
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
                            entry.insert(String::from("total cost"), ((audit.input_cost + audit.output_cost) as f64 / 1_000_000.0).into());
                            entry.insert(String::from("input cost"), (audit.input_cost as f64 / 1_000_000.0).into());
                            entry.insert(String::from("output cost"), (audit.output_cost as f64 / 1_000_000.0).into());
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
                            println!("    total cost:  {:.03}$", (audit.input_cost + audit.output_cost) as f64 / 1_000_000.0);
                            println!("    input cost:  {:.03}$", audit.input_cost as f64 / 1_000_000.0);
                            println!("    output cost: {:.03}$", audit.output_cost as f64 / 1_000_000.0);
                        }
                    }
                }
            }
        },
        Some("cat-file") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--json"])
                .short_flag(&["--json"])
                .args(ArgType::String, ArgCount::Exact(1))  // uid or path
                .parse(&args, 2)?;

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
                    println!("{}", serde_json::to_string_pretty(&into_multi_modal_contents(&chunk.data, &chunk.images))?);
                }

                else {
                    println!("{}", chunk.data);
                }
            }

            else if let Some((_, uid)) = query_result.get_processed_file() {
                let chunk = index.get_merged_chunk_of_file(uid)?;

                if json_mode {
                    println!("{}", serde_json::to_string_pretty(&chunk.raw_data)?);
                }

                else {
                    println!("{}", chunk.human_data);
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

            else if let Some(query_uid) = query_result.get_query_history_uid() {
                let query_turns = index.get_query_schema(query_uid)?;
                let query_turns = render_query_turns(&query_turns);

                if json_mode {
                    println!("{}", serde_json::to_string_pretty(&query_turns)?);
                }

                else {
                    for turn in query_turns.iter() {
                        println!("<|{}|>", turn.role);
                        println!("");
                        println!("{}", turn.content);
                        println!("");
                    }
                }
            }

            else {
                return Err(Error::UidQueryError(format!("There's no chunk/file/image that matches `{}`.", query[0])));
            }
        },
        Some("check") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--recover"])
                .parse(&args, 2)?;

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
                .flag_with_default(&["--ii", "--no-ii"])
                .short_flag(&["--quiet"])
                .args(ArgType::String, ArgCount::Geq(1))  // url and path
                .parse(&args, 2)?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/clone.txt"));
                return Ok(());
            }

            if root_dir.is_ok() {
                return Err(Error::CannotClone(String::from("You're already inside a knowledge-base. You cannot clone another knowledge-base here.")));
            }

            let args = parsed_args.get_args();
            let quiet = parsed_args.get_flag(0).is_some();
            let ii = parsed_args.get_flag(1).unwrap() == "--ii";
            Index::clone(
                args[0].clone(),
                args.get(1).map(|s| s.to_string()),
                ii,
                quiet,
            ).await?;
            return Ok(());
        },
        Some("config") => {
            let parsed_args = ArgParser::new().parse(&args, 2);

            // `ArgParser.parse()` fails unless it's `rag config --help`
            match parsed_args {
                Ok(parsed_args) if parsed_args.show_help() => {
                    println!("{}", include_str!("../docs/commands/config.txt"));
                    return Ok(());
                },
                _ => {},
            }

            let mut index = Index::load(root_dir?, LoadMode::OnlyJson)?;

            match args.get(2).map(|s| s.as_str()) {
                Some("--get") => {
                    let parsed_args = ArgParser::new()
                        .optional_flag(&["--json"])
                        .short_flag(&["--json"])
                        .args(ArgType::String, ArgCount::Exact(1))
                        .parse(&args, 3)?;

                    let args = parsed_args.get_args_exact(1)?;
                    let json_mode = parsed_args.get_flag(0).is_some();

                    let s = match index.get_config_by_key(args[0].clone())? {
                        Value::String(s) => if json_mode {
                            format!("{s:?}")
                        } else {
                            s.to_string()
                        },
                        v => v.to_string(),
                    };
                    println!("{s}");
                },
                Some("--set") => {
                    let parsed_args = ArgParser::new()
                        .args(ArgType::String, ArgCount::Exact(2))
                        .parse(&args, 3)?;
                    let args = parsed_args.get_args_exact(2)?;
                    let key = args[0].clone();
                    let value = args[1].clone();

                    // QoL improvement: it warns if the user typed a wrong model name.
                    if &key == "model" {
                        let models = Index::list_models(
                            &join3(
                                &index.root_dir,
                                INDEX_DIR_NAME,
                                MODEL_FILE_NAME,
                            )?,
                            &|_| true,  // no filter
                            &|model| model,  // no map
                            &|model| model.name.to_string(),
                        )?;

                        if let Err(e @ ragit_api::Error::InvalidModelName { .. }) = get_model_by_name(&models, &value) {
                            return Err(e.into());
                        }
                    }

                    let previous_value = index.set_config_by_key(key.clone(), value.clone())?;

                    match previous_value {
                        Some(prev) => {
                            println!("set `{key}`: `{prev}` -> `{value}`");
                        },
                        None => {
                            println!("set `{key}`: `{value}`");
                        },
                    }
                },
                Some("--get-all") => {
                    let parsed_args = ArgParser::new()
                        .optional_flag(&["--json"])
                        .short_flag(&["--json"])
                        .parse(&args, 3)?;

                    let json_mode = parsed_args.get_flag(0).is_some();
                    let mut kv = index.get_all_configs()?;
                    kv.sort_by_key(|(k, _)| k.to_string());

                    if json_mode {
                        println!("{}", '{');

                        for (i, (k, v)) in kv.iter().enumerate() {
                            println!(
                                "    {k:?}: {v}{}",
                                if i != kv.len() - 1 { "," } else { "" },
                            );
                        }

                        println!("{}", '}');
                    }

                    else {
                        for (k, v) in kv.iter() {
                            println!("{k}: {v}");
                        }
                    }
                },
                Some(flag) => {
                    return Err(Error::CliError {
                        message: format!("Unknown flag: `{flag}`. Valid flags are --get | --get-all | --set."),
                        span: Span::Exact(2).render(&args, 2),
                    });
                },
                None => {
                    return Err(Error::CliError {
                        message: String::from("Flag `--get | --get-all | --set` is missing."),
                        span: Span::End.render(&args, 2),
                    });
                },
            }
        },
        Some("extract-keywords") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--full-schema"])
                .optional_flag(&["--json"])
                .short_flag(&["--json"])
                .args(ArgType::String, ArgCount::Exact(1))  // query
                .parse(&args, 2)?;

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
            let parsed_args = ArgParser::new()
                .flag(&["--logs", "--images", "--audit", "--all"])
                .parse(&args, 2)?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/gc.txt"));
                return Ok(());
            }

            let mut index = Index::load(root_dir?, LoadMode::OnlyJson)?;

            match parsed_args.get_flag(0).unwrap().as_str() {
                "--logs" => {
                    let removed = index.gc_logs()?;
                    println!("removed {removed} log files");
                },
                "--images" => {
                    let removed = index.gc_images()?;
                    println!("removed {removed} images");
                },
                "--audit" => {
                    index.gc_audit()?;
                    println!("removed audit logs");
                },
                "--all" => {
                    let removed_logs = index.gc_logs()?;
                    let removed_images = index.gc_images()?;
                    index.gc_audit()?;
                    println!("removed {removed_logs} log files, {removed_images} images and audit logs");
                },
                _ => unreachable!(),
            }
        },
        // It matches `rag help` and `rag --help`.
        Some("help" | "--help") => {
            let parsed_args = ArgParser::new()
                .args(ArgType::String, ArgCount::Leq(1))  // command
                .parse(&args, 2)?;

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
                Some("pdl-format") => {
                    println!("{}", include_str!("../docs/pdl_format.md"));
                },
                Some("pipeline") => {
                    println!("{}", include_str!("../docs/pipeline.md"));
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
                    println!("{}", include_str!("../docs/intro.txt"));
                },
            }
        },
        Some("ii-build") | Some("build-ii") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--quiet"])
                .short_flag(&["--quiet"])
                .parse(&args, 2)?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/ii-build.txt"));
                return Ok(());
            }

            let mut index = Index::load(root_dir?, LoadMode::QuickCheck)?;
            let quiet = parsed_args.get_flag(0).is_some();
            index.build_ii(quiet)?;
        },
        Some("ii-reset") | Some("reset-ii") => {
            let parsed_args = ArgParser::new().parse(&args, 2)?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/ii-reset.txt"));
                return Ok(());
            }

            let mut index = Index::load(root_dir?, LoadMode::QuickCheck)?;
            index.reset_ii()?;
        },
        Some("ii-status") => {
            let parsed_args = ArgParser::new().parse(&args, 2)?;

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
            let parsed_args = ArgParser::new().parse(&args, 2)?;

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
                .arg_flag_with_default("--abbrev", "9", ArgType::integer_between(Some(4), Some(64)))
                .short_flag(&["--json"])
                .args(ArgType::String, ArgCount::Any)  // uid or path
                .parse(&args, 2)?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/ls-chunks.txt"));
                return Ok(());
            }

            let uid_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--uid-only";
            let stat_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--stat-only";
            let json_mode = parsed_args.get_flag(1).is_some();
            let abbrev = parsed_args.arg_flags.get("--abbrev").unwrap().parse::<usize>().unwrap();
            let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let args = parsed_args.get_args();

            let chunk_uids = if args.is_empty() {
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
                    &|chunk: &ChunkSchema| chunk.source.sortable_string(),  // sort by source
                )?
            } else {
                let query = index.uid_query(&args, UidQueryConfig::new().file_or_chunk_only())?;
                let mut chunk_uids = query.get_chunk_uids();

                if chunk_uids.is_empty() {
                    for file_uid in query.get_file_uids() {
                        let uids = index.get_chunks_of_file(file_uid)?;

                        for uid in uids.iter() {
                            chunk_uids.push(*uid);
                        }
                    }
                }

                if chunk_uids.is_empty() {
                    return Err(Error::UidQueryError(format!("There's no chunk/file that matches `{}`.", args.join(" "))));
                }

                if !uid_only {
                    if !json_mode {
                        println!("{} chunks", chunk_uids.len());
                    }

                    else if stat_only {
                        println!("{}\"chunks\": {}{}", "{", chunk_uids.len(), "}");
                    }
                }

                if stat_only {
                    return Ok(());
                }

                // In this branch, the user is likely to be looking for a specific chunk/file,
                // and `chunk_uids` is likely to be small. So, it'd be okay to load everything to memory.
                let mut chunks = Vec::with_capacity(chunk_uids.len());

                for chunk_uid in chunk_uids.iter() {
                    chunks.push(index.get_chunk_by_uid(*chunk_uid)?);
                }

                chunks.sort_by_key(|chunk| chunk.sortable_string());
                chunks.iter().map(|chunk| chunk.uid).collect()
            };

            if json_mode {
                if uid_only {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &chunk_uids.iter().map(
                                |uid| uid.abbrev(abbrev)
                            ).collect::<Vec<_>>(),
                        )?,
                    );
                }

                else {
                    let mut chunks = Vec::with_capacity(chunk_uids.len());

                    for chunk_uid in chunk_uids.iter() {
                        chunks.push(index.get_chunk_by_uid(*chunk_uid)?);
                    }

                    println!(
                        "{}",
                        serde_json::to_string_pretty(&chunks.prettify()?)?,
                    );
                }
            }

            else {
                for chunk_uid in chunk_uids.iter() {
                    if uid_only {
                        println!("{}", chunk_uid.abbrev(abbrev));
                        continue;
                    }

                    let chunk = index.get_chunk_by_uid(*chunk_uid)?;
                    println!("----------");
                    println!("{}", chunk.render_source());
                    println!("uid: {}", chunk.uid.abbrev(abbrev));
                    println!("character_len: {}", chunk.char_len);
                    println!("title: {}", chunk.title);
                    println!("summary: {}", chunk.summary);
                }
            }
        },
        Some("ls-files") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--name-only", "--uid-only", "--stat-only"])
                .optional_flag(&["--staged", "--processed"])
                .optional_flag(&["--json"])
                .arg_flag_with_default("--abbrev", "9", ArgType::integer_between(Some(4), Some(64)))
                .short_flag(&["--json"])
                .alias("--cached", "--staged")
                .args(ArgType::String, ArgCount::Any)  // uid or path
                .parse(&args, 2)?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/ls-files.txt"));
                return Ok(());
            }

            let name_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--name-only";
            let uid_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--uid-only";
            let stat_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--stat-only";
            let mut staged = parsed_args.get_flag(1).unwrap_or(String::from("--staged")) == "--staged";
            let mut processed = parsed_args.get_flag(1).unwrap_or(String::from("--processed")) == "--processed";
            let json_mode = parsed_args.get_flag(2).is_some();
            let abbrev = parsed_args.arg_flags.get("--abbrev").unwrap().parse::<usize>().unwrap();
            let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let args = parsed_args.get_args();

            // If it's `--uid-only` there's no point of listing staged files because
            // staged files do not have uids.
            if uid_only {
                staged = false;
                processed = true;
            }

            let files = if args.is_empty() {
                if !uid_only && !name_only {
                    let staged_files = if staged { index.staged_files.len() } else { 0 };
                    let processed_files = if processed { index.processed_files.len() } else { 0 };

                    if json_mode && stat_only {
                        println!(
                            "{}\"total files\": {}, \"staged files\": {}, \"processed files\": {}{}",
                            "{",
                            staged_files + processed_files,
                            staged_files,
                            processed_files,
                            "}",
                        );
                    }

                    else if !json_mode {
                        println!(
                            "{} total files, {} staged files, {} processed files",
                            staged_files + processed_files,
                            staged_files,
                            processed_files,
                        );
                    }
                }

                if stat_only {
                    return Ok(());
                }

                index.list_files(
                    // respect `--staged` and `--processed` options
                    &|f| staged && !f.is_processed || processed && f.is_processed,

                    // sort by path
                    &|f| f.path.to_string(),
                )?
            } else {
                let query = index.uid_query(&args, UidQueryConfig::new().file_only())?;
                let mut files = vec![];
                let mut processed_files_len = 0;
                let mut staged_files_len = 0;

                if processed {
                    for (_, uid) in query.get_processed_files() {
                        processed_files_len += 1;
                        files.push(UidOrStagedFile::Uid(uid));
                    }
                }

                if staged {
                    for path in query.get_staged_files() {
                        staged_files_len += 1;
                        files.push(UidOrStagedFile::StagedFile(path));
                    }
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
                if uid_only {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &files.iter().map(
                                |file| match file {
                                    UidOrStagedFile::Uid(uid) => uid.abbrev(abbrev),
                                    UidOrStagedFile::StagedFile(_) => unreachable!(),
                                }
                            ).collect::<Vec<_>>(),
                        )?,
                    );
                }

                else {
                    let mut file_schemas = Vec::with_capacity(files.len());

                    for file in files.iter() {
                        let file = match file {
                            UidOrStagedFile::Uid(uid) => index.get_file_schema(None, Some(*uid))?,
                            UidOrStagedFile::StagedFile(file) => index.get_file_schema(Some(file.to_string()), None)?,
                        };

                        file_schemas.push(file);
                    }

                    if name_only {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(
                                &file_schemas.iter().map(
                                    |file| file.path.to_string()
                                ).collect::<Vec<_>>(),
                            )?,
                        );
                    }

                    else {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&file_schemas.prettify()?)?,
                        );
                    }
                }
            }

            else {
                for file in files.iter() {
                    if uid_only {
                        println!(
                            "{}",
                            match file {
                                UidOrStagedFile::Uid(uid) => uid.abbrev(abbrev),
                                UidOrStagedFile::StagedFile(_) => unreachable!(),
                            },
                        );
                        continue;
                    }

                    let file = match file {
                        UidOrStagedFile::Uid(uid) => index.get_file_schema(None, Some(*uid))?,
                        UidOrStagedFile::StagedFile(file) => index.get_file_schema(Some(file.to_string()), None)?,
                    };

                    if name_only {
                        println!("{}", file.path);
                        continue;
                    }

                    println!("--------");
                    println!("name: {}{}", file.path, if file.is_processed { String::new() } else { String::from(" (not processed yet)") });

                    if file.is_processed {
                        println!("length: {}", file.length);
                        println!("uid: {}", file.uid.abbrev(abbrev));
                        println!("chunks: {}", file.chunks);
                    }
                }
            }
        },
        Some("ls-images") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--uid-only", "--stat-only"])
                .optional_flag(&["--json"])
                .arg_flag_with_default("--abbrev", "9", ArgType::integer_between(Some(4), Some(64)))
                .short_flag(&["--json"])
                .args(ArgType::String, ArgCount::Any)  // uid or path
                .parse(&args, 2)?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/ls-images.txt"));
                return Ok(());
            }

            let uid_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--uid-only";
            let stat_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--stat-only";
            let json_mode = parsed_args.get_flag(1).is_some();
            let abbrev = parsed_args.arg_flags.get("--abbrev").unwrap().parse::<usize>().unwrap();
            let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let args = parsed_args.get_args();

            let image_uids = if args.is_empty() {
                let result = index.list_images(
                    &|_| true,  // no filter
                    &|image| image.uid,  // sort by uid
                )?;

                result
            } else {
                let query = index.uid_query(&args, UidQueryConfig::new().no_query_history())?;
                let mut image_uids = vec![];
                let mut matched_files = false;
                let mut matched_chunks = false;

                for (_, uid) in query.get_processed_files() {
                    matched_files = true;

                    for image_uid in index.get_images_of_file(uid)? {
                        image_uids.push(image_uid);
                    }
                }

                for uid in query.get_chunk_uids() {
                    matched_chunks = true;
                    let chunk = index.get_chunk_by_uid(uid)?;

                    for image_uid in chunk.images {
                        image_uids.push(image_uid);
                    }
                }

                for image_uid in query.get_image_uids() {
                    image_uids.push(image_uid);
                }

                if image_uids.is_empty() {
                    if matched_files {
                        return Err(Error::UidQueryError(format!("There is a file that matches `{}`, but it has no images.", args.join(" "))));
                    }

                    else if matched_chunks {
                        return Err(Error::UidQueryError(format!("There is a chunk that matches `{}`, but it has no images.", args.join(" "))));
                    }

                    else {
                        return Err(Error::UidQueryError(format!("There's no chunk/file/image that matches `{}`.", args.join(" "))));
                    }
                }

                // dedup and sort
                image_uids = image_uids.into_iter().collect::<HashSet<_>>().into_iter().collect();
                image_uids.sort();

                image_uids
            };

            if uid_only {
                if json_mode {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &image_uids.iter().map(
                                |uid| uid.abbrev(abbrev)
                            ).collect::<Vec<_>>(),
                        )?,
                    );
                }

                else {
                    for uid in image_uids.iter() {
                        println!("{}", uid.abbrev(abbrev));
                    }
                }
            }

            else if stat_only {
                if json_mode {
                    println!("{}\"images\": {}{}", '{', image_uids.len(), '}');
                }

                else {
                    println!("{} images", image_uids.len());
                }
            }

            else {
                if json_mode {
                    let mut images = Vec::with_capacity(image_uids.len());

                    for image_uid in image_uids.iter() {
                        images.push(index.get_image_schema(*image_uid, false)?);
                    }

                    println!(
                        "{}",
                        serde_json::to_string_pretty(&images.prettify()?)?,
                    );
                }

                else {
                    println!("{} images", image_uids.len());

                    for image_uid in image_uids.iter() {
                        let image = index.get_image_schema(*image_uid, false)?;

                        println!("--------");
                        println!("uid: {}", image.uid.abbrev(abbrev));
                        println!("explanation: {}", image.explanation);
                        println!("extracted_text: {}", image.extracted_text);
                        println!("size: {}", image.size);
                    }
                }
            }
        },
        Some("ls-models") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--name-only", "--stat-only"])
                .optional_flag(&["--selected"])
                .optional_flag(&["--json"])
                .short_flag(&["--json"])
                .args(ArgType::String, ArgCount::Leq(1))
                .parse(&args, 2)?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/ls-models.txt"));
                return Ok(());
            }

            let name_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--name-only";
            let stat_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--stat-only";
            let selected_only = parsed_args.get_flag(1).is_some();
            let json_mode = parsed_args.get_flag(2).is_some();
            let args = parsed_args.get_args();
            let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let mut models = Index::list_models(
                &join3(
                    &index.root_dir,
                    INDEX_DIR_NAME,
                    MODEL_FILE_NAME,
                )?,
                &|_| true,  // no filter
                &|model| model,  // no map
                &|model| model.name.to_string(),
            )?;

            if selected_only {
                if !args.is_empty() {
                    return Err(Error::CliError {
                        message: String::from("You cannot use `--selected` option with a model name."),
                        span: None,
                    });
                }

                models = match get_model_by_name(&models, &index.api_config.model) {
                    Ok(model) => vec![model.clone()],
                    Err(_) => match index.find_lowest_cost_model() {
                        Some(model) => vec![model.clone()],
                        None => vec![],
                    },
                };
            }

            else if let Some(model) = args.get(0) {
                models = match get_model_by_name(&models, model) {
                    Ok(model) => vec![model.clone()],
                    Err(ragit_api::Error::InvalidModelName { candidates, .. }) => models.iter().filter(
                        |model| candidates.contains(&model.name)
                    ).map(
                        |model| model.clone()
                    ).collect(),
                    Err(_) => vec![],
                };
            }

            // NOTE: there's a duplicate of this code block at `model`'s implementation
            //       be aware when you edit this code
            if !json_mode && !name_only {
                println!("{} models", models.len());
            }

            if stat_only {
                if json_mode {
                    println!("{} \"models\": {} {}", "{", models.len(), "}");
                }

                return Ok(());
            }

            if json_mode {
                if name_only {
                    println!("{}", serde_json::to_string_pretty(&models.iter().map(|model| &model.name).collect::<Vec<_>>())?);
                }

                else {
                    println!("{}", serde_json::to_string_pretty(&models.iter().map(
                        |model| vec![
                            (String::from("name"), model.name.clone().into()),
                            (String::from("api_provider"), model.api_provider.to_string().into()),
                            (String::from("api_key_env_var"), model.api_env_var.clone().into()),
                            (String::from("can_read_images"), model.can_read_images.into()),
                            (String::from("dollars_per_1b_input_tokens"), model.dollars_per_1b_input_tokens.into()),
                            (String::from("dollars_per_1b_output_tokens"), model.dollars_per_1b_output_tokens.into()),
                        ].into_iter().collect::<Map<String, Value>>()
                    ).collect::<Vec<_>>())?);
                }
            }

            else {
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
            }
        },
        Some("ls-queries") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--uid-only", "--stat-only", "--content-only"])
                .optional_flag(&["--json"])
                .arg_flag_with_default("--abbrev", "9", ArgType::integer_between(Some(4), Some(64)))
                .short_flag(&["--json"])
                .args(ArgType::String, ArgCount::Any)  // uid
                .parse(&args, 2)?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/ls-queries.txt"));
                return Ok(());
            }

            let uid_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--uid-only";
            let stat_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--stat-only";
            let content_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--content-only";
            let json_mode = parsed_args.get_flag(1).is_some();
            let abbrev = parsed_args.arg_flags.get("--abbrev").unwrap().parse::<usize>().unwrap();
            let index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let args = parsed_args.get_args();

            let queries = if args.is_empty() {
                index.list_queries(
                    &|_| true,  // no filter
                    &|q| -q[0].timestamp,  // ORDER BY timestamp DESC
                )?
            } else {
                let mut query_uids = index.uid_query(&args, UidQueryConfig::new().query_history_only())?.query_histories;

                if query_uids.is_empty() {
                    return Err(Error::UidQueryError(format!("There's no query history that matches `{}`.", args.join(" "))));
                }

                // The last 8 characters of uid is a timestamp.
                query_uids.sort_by_key(
                    |uid| u32::MAX - u32::from_str_radix(uid.to_string().get(56..64).unwrap(), 16).unwrap()
                );
                query_uids
            };

            if stat_only {
                if json_mode {
                    println!("{}\"queries\": {}{}", "{", queries.len(), "}");
                }

                else {
                    println!("{} queries", queries.len());
                }

                return Ok(());
            }

            if uid_only {
                if json_mode {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&queries.iter().map(|uid| uid.abbrev(abbrev)).collect::<Vec<_>>())?,
                    );
                }

                else {
                    println!(
                        "{}",
                        queries.iter().map(|uid| uid.abbrev(abbrev)).collect::<Vec<_>>().join("\n"),
                    );
                }
            }

            else {
                let mut json_array: Vec<Value> = if json_mode {
                    Vec::with_capacity(queries.len())
                } else {
                    vec![]
                };

                for uid in queries.iter() {
                    let query = index.get_query_by_uid(*uid)?;

                    // TODO: it has to include the uids of queries
                    if json_mode {
                        if content_only {
                            json_array.push(serde_json::to_value(render_query_turns(&query))?);
                        }

                        else {
                            json_array.push(query.prettify()?);
                        }
                    }

                    else {
                        if content_only {
                            for turn in render_query_turns(&query).iter() {
                                println!("<|{}|>", turn.role);
                                println!("");
                                println!("{}", turn.content);
                                println!("");
                            }
                        }

                        else {
                            println!("--------");
                            println!("uid: {}", uid.abbrev(abbrev));
                            println!(
                                "query: {}",
                                if query[0].query.len() > 80 {
                                    format!("{}...", String::from_utf8_lossy(&query[0].query.replace("\n", " ").as_bytes()[..80]))
                                } else {
                                    query[0].query.replace("\n", " ")
                                },
                            );
                            println!("model: {}", query[0].response.model);
                            println!(
                                "created_at: {}",
                                DateTime::from_timestamp(query[0].timestamp, 0).map(
                                    |t| t.to_rfc3339()
                                ).unwrap_or(String::from("error")),
                            );
                        }
                    }
                }

                if json_mode {
                    println!("{}", serde_json::to_string_pretty(&json_array)?);
                }
            }
        },
        Some("ls-terms") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--term-only", "--stat-only"])
                .optional_flag(&["--json"])
                .short_flag(&["--json"])
                .args(ArgType::String, ArgCount::Any)  // uid or path
                .parse(&args, 2)?;

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
                .optional_arg_flag("--prefix", ArgType::String)
                .optional_flag(&["--quiet"])
                .short_flag(&["--force", "--quiet"])
                .args(ArgType::String, ArgCount::Geq(1))  // path
                .parse(&args, 2)?;

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
            let parsed_args = ArgParser::new().parse(&args, 2);

            // `ArgParser.parse()` fails unless it's `rag meta --help`
            match parsed_args {
                Ok(parsed_args) if parsed_args.show_help() => {
                    println!("{}", include_str!("../docs/commands/meta.txt"));
                    return Ok(());
                },
                _ => {},
            }

            let mut index = Index::load(root_dir?, LoadMode::OnlyJson)?;

            match args.get(2).map(|s| s.as_str()) {
                Some("--get") => {
                    let parsed_args = ArgParser::new()
                        .optional_flag(&["--json"])
                        .short_flag(&["--json"])
                        .args(ArgType::String, ArgCount::Exact(1))
                        .parse(&args, 3)?;

                    let args = parsed_args.get_args_exact(1)?;
                    let key = args[0].to_string();
                    let json_mode = parsed_args.get_flag(0).is_some();

                    if let Some(value) = index.get_meta_by_key(key.clone())? {
                        if json_mode {
                            println!("{value:?}");
                        }

                        else {
                            println!("{value}");
                        }
                    }

                    else {
                        return Err(Error::NoSuchMeta(key));
                    }
                },
                Some("--get-all") => {
                    let parsed_args = ArgParser::new()
                        .optional_flag(&["--json"])
                        .short_flag(&["--json"])
                        .parse(&args, 3)?;

                    let json_mode = parsed_args.get_flag(0).is_some();
                    let all = index.get_all_meta()?;

                    if json_mode {
                        println!("{}", serde_json::to_string_pretty(&all)?);
                    }

                    else {
                        for (k, v) in all.iter() {
                            println!("{k}: {v}");
                        }
                    }
                },
                // git uses the term "add"
                Some("--set" | "--add") => {
                    let parsed_args = ArgParser::new()
                        .args(ArgType::String, ArgCount::Exact(2))
                        .parse(&args, 3)?;
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
                // git uses the term "unset"
                Some("--remove" | "--unset") => {
                    let parsed_args = ArgParser::new()
                        .args(ArgType::String, ArgCount::Exact(1))
                        .parse(&args, 3)?;
                    let key = &parsed_args.get_args_exact(1)?[0];
                    let prev_value = index.remove_meta_by_key(key.to_string())?;

                    println!("metadata unset `{key}`: `{prev_value}`");
                },
                // git uses the term "--unset-all"
                Some("--remove-all" | "--unset-all") => {
                    ArgParser::new()
                        .parse(&args, 3)?;

                    index.remove_all_meta()?;
                    println!("metadata removed");
                },
                Some(flag) => {
                    return Err(Error::CliError {
                        message: format!("Unknown flag: `{flag}`. Valid flags are --get | --get-all | --set | --remove | --remove-all."),
                        span: Span::Exact(2).render(&args, 2),
                    });
                },
                None => {
                    return Err(Error::CliError {
                        message: String::from("Flag `--get | --get-all | --set | --remove | --remove-all` is missing."),
                        span: Span::End.render(&args, 2),
                    });
                },
            }
        },
        Some("migrate") => {
            let parsed_args = ArgParser::new().parse(&args, 2)?;

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
        Some("model") => {
            let parsed_args = ArgParser::new().parse(&args, 2);

            // `ArgParser.parse()` fails unless it's `rag model --help`
            match parsed_args {
                Ok(parsed_args) if parsed_args.show_help() => {
                    println!("{}", include_str!("../docs/commands/model.txt"));
                    return Ok(());
                },
                _ => {},
            }

            match args.get(2).map(|s| s.as_str()) {
                Some("--search") => {
                    let parsed_args = ArgParser::new()
                        .arg_flag_with_default("--remote", "https://ragit.baehyunsol.com", ArgType::String)
                        .optional_flag(&["--name-only", "--stat-only"])
                        .optional_flag(&["--json"])
                        .short_flag(&["--json"])
                        .args(ArgType::String, ArgCount::Exact(1))
                        .parse(&args, 3)?;

                    let name_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--name-only";
                    let stat_only = parsed_args.get_flag(0).unwrap_or(String::new()) == "--stat-only";
                    let json_mode = parsed_args.get_flag(1).is_some();
                    let remote = parsed_args.arg_flags.get("--remote").unwrap();
                    let keyword = parsed_args.get_args_exact(1)?[0].to_string();
                    let models = Index::search_remote_models(&keyword, &remote).await?;

                    // TODO: it's a duplicate of `ls-models`' code
                    //       can we refactor this?
                    if !json_mode && !name_only {
                        println!("{} models", models.len());
                    }

                    if stat_only {
                        if json_mode {
                            println!("{} \"models\": {} {}", "{", models.len(), "}");
                        }

                        return Ok(());
                    }

                    if json_mode {
                        if name_only {
                            println!("{}", serde_json::to_string_pretty(&models.iter().map(|model| &model.name).collect::<Vec<_>>())?);
                        }

                        else {
                            println!("{}", serde_json::to_string_pretty(&models.iter().map(
                                |model| vec![
                                    (String::from("name"), model.name.clone().into()),
                                    (String::from("api_provider"), model.api_provider.to_string().into()),
                                    (String::from("api_key_env_var"), model.api_env_var.clone().into()),
                                    (String::from("can_read_images"), model.can_read_images.into()),
                                    (String::from("dollars_per_1b_input_tokens"), model.dollars_per_1b_input_tokens.into()),
                                    (String::from("dollars_per_1b_output_tokens"), model.dollars_per_1b_output_tokens.into()),
                                ].into_iter().collect::<Map<String, Value>>()
                            ).collect::<Vec<_>>())?);
                        }
                    }

                    else {
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
                    }
                },
                Some("--fetch") => {
                    let parsed_args = ArgParser::new()
                        .arg_flag_with_default("--remote", "https://ragit.baehyunsol.com", ArgType::String)
                        .optional_flag(&["--all"])
                        .optional_flag(&["--existing-only"])
                        .optional_flag(&["--quiet"])
                        .short_flag(&["--all", "--quiet"])
                        .args(ArgType::String, ArgCount::Leq(1))
                        .parse(&args, 3)?;

                    let mut index = Index::load(root_dir?, LoadMode::OnlyJson)?;
                    let all = parsed_args.get_flag(0).is_some();
                    let existing_only = parsed_args.get_flag(1).is_some();
                    let quiet = parsed_args.get_flag(2).is_some();
                    let model_name = parsed_args.get_args().get(0).map(|model| model.to_string());
                    let remote = parsed_args.arg_flags.get("--remote").unwrap();

                    let result = if let Some(model_name) = model_name {
                        if all {
                            return Err(Error::CliError {
                                message: String::from("You cannot use `--all` option with a model name."),
                                span: None,
                            });
                        }

                        index.fetch_remote_models(&model_name, existing_only, &remote).await?
                    }

                    else if all {
                        index.fetch_all_remote_models(existing_only, &remote).await?
                    }

                    else {
                        return Err(Error::CliError {
                            message: String::from("Please specify which model to fetch."),
                            span: Span::End.render(&args, 2),
                        });
                    };

                    if !quiet {
                        println!("fetched {} new models, updated {} models", result.fetched, result.updated);
                    }
                },
                Some("--remove") => {
                    let parsed_args = ArgParser::new()
                        .optional_flag(&["--all"])
                        .short_flag(&["--all"])
                        .args(ArgType::String, ArgCount::Leq(1))
                        .parse(&args, 3)?;

                    let mut index = Index::load(root_dir?, LoadMode::OnlyJson)?;
                    let all = parsed_args.get_flag(0).is_some();
                    let model_name = parsed_args.get_args().get(0).map(|model| model.to_string());

                    if let Some(model_name) = model_name {
                        if all {
                            return Err(Error::CliError {
                                message: String::from("You cannot use `--all` option with a model name."),
                                span: None,
                            });
                        }

                        index.remove_local_model(&model_name)?;
                    }

                    else if all {
                        index.remove_all_local_models()?;
                    }

                    else {
                        return Err(Error::CliError {
                            message: String::from("Please specify which model to remove."),
                            span: Span::End.render(&args, 2),
                        });
                    }
                },
                Some(flag) => {
                    return Err(Error::CliError {
                        message: format!("Unknown flag: `{flag}`. Valid flags are --search | --update | --remove."),
                        span: Span::Exact(2).render(&args, 2),
                    });
                },
                None => {
                    return Err(Error::CliError {
                        message: String::from("Flag `--search | --update | --remove` is missing."),
                        span: Span::End.render(&args, 2),
                    });
                },
            }
        },
        // TODO: maybe create a function for this? I don't want `main.rs` to be too bloated
        Some("pdl") => {
            let parsed_args = ArgParser::new()
                .flag_with_default(&["--strict", "--no-strict"])
                .optional_arg_flag("--model", ArgType::String)
                .optional_arg_flag("--models", ArgType::String)
                .optional_arg_flag("--context", ArgType::String)
                .optional_arg_flag("--log", ArgType::String)
                .optional_arg_flag("--schema", ArgType::String)
                .args(ArgType::String, ArgCount::Exact(1))  // path
                .parse(&args, 2)?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/pdl.txt"));
                return Ok(());
            }

            // `index.json` will help us find models, but it's not necessary
            let index = root_dir.map(|root_dir| Index::load(root_dir, LoadMode::OnlyJson));
            let pdl_at = parsed_args.get_args_exact(1)?[0].clone();
            let strict_mode = parsed_args.get_flag(0).unwrap() == "--strict";
            let models = match parsed_args.arg_flags.get("--models") {
                Some(path) => {
                    let m = read_string(path)?;
                    let models_raw = serde_json::from_str::<Vec<ModelRaw>>(&m)?;
                    let mut models = Vec::with_capacity(models_raw.len());

                    for model_raw in models_raw.iter() {
                        models.push(Model::try_from(model_raw)?);
                    }

                    models
                },

                // looks for `models.json` with best-effort
                None => match &index {
                    Ok(Ok(index)) => index.models.clone(),
                    _ => {
                        let models_raw = Index::get_initial_models()?;
                        let mut models = Vec::with_capacity(models_raw.len());

                        for model_raw in models_raw.iter() {
                            models.push(Model::try_from(model_raw)?);
                        }

                        models
                    },
                },
            };
            let model = match parsed_args.arg_flags.get("--model") {
                Some(model) => get_model_by_name(&models, model)?,
                None => match &index {
                    Ok(Ok(index)) => get_model_by_name(&models, &index.api_config.model)?,
                    _ => match Index::load_config_from_home::<Value>("api.json") {
                        Ok(Some(Value::Object(api_config))) => match api_config.get("model") {
                            Some(Value::String(model)) => get_model_by_name(&models, model)?,
                            _ => { return Err(Error::ModelNotSelected); },
                        },
                        _ => { return Err(Error::ModelNotSelected); },
                    },
                },
            };
            let context = match parsed_args.arg_flags.get("--context") {
                Some(path) => {
                    let s = read_string(path)?;
                    serde_json::from_str::<Value>(&s)?
                },

                // an empty context
                None => Value::Object(serde_json::Map::new()),
            };
            let (dump_pdl_at, dump_json_at) = match parsed_args.arg_flags.get("--log") {
                Some(log_at) => {
                    let now = Local::now();

                    if !exists(log_at) {
                        create_dir(log_at)?;
                    }

                    (
                        Some(join(
                            log_at,
                            &format!("{}.pdl", now.to_rfc3339()),
                        )?),
                        Some(log_at.to_string()),
                    )
                },
                None => (None, None),
            };
            let arg_schema = match parsed_args.arg_flags.get("--schema") {
                Some(schema) => Some(ragit_pdl::parse_schema(schema)?),
                None => None,
            };
            let Pdl { messages, schema: pdl_schema } = ragit_pdl::parse_pdl_from_file(
                &pdl_at,
                &tera::Context::from_value(context)?,
                strict_mode,
            )?;
            let schema = match (pdl_schema, arg_schema) {
                (_, Some(schema)) => Some(schema),
                (Some(schema), _) => Some(schema),
                _ => None,
            };
            let dump_api_usage_at = match &index {
                Ok(Ok(index)) => index.api_config.dump_api_usage_at(&index.root_dir, "pdl"),
                _ => None,
            };

            let request = ragit_api::Request {
                messages,
                schema: schema.clone(),
                model: model.clone(),
                dump_pdl_at,
                dump_json_at,
                dump_api_usage_at,

                // TODO: do these have to be configurable?
                temperature: None,
                timeout: Some(model.api_timeout * 1_000),
                max_retry: 3,
                max_tokens: None,
                sleep_between_retries: 10_000,
                frequency_penalty: None,
                schema_max_try: 3,
            };

            let response = match schema {
                Some(schema) => {
                    let result = request.send_and_validate::<serde_json::Value>(serde_json::Value::Null).await?;
                    render_pdl_schema(&schema, &result)?
                },
                None => request.send().await?.get_message(0).unwrap().to_string(),
            };

            println!("{response}");
        },
        Some("pull") => {
            let parsed_args = ArgParser::new()
                .flag_with_default(&["--no-configs", "--configs"])
                .flag_with_default(&["--no-prompts", "--prompts"])
                .optional_flag(&["--quiet"])
                .flag_with_default(&["--ii", "--no-ii"])
                .short_flag(&["--quiet"])
                .parse(&args, 2)?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/pull.txt"));
                return Ok(());
            }

            let index = Index::load(root_dir?, LoadMode::QuickCheck)?;
            let include_configs = parsed_args.get_flag(0).unwrap() == "--configs";
            let include_prompts = parsed_args.get_flag(1).unwrap() == "--prompts";
            let quiet = parsed_args.get_flag(2).is_some();
            let ii = parsed_args.get_flag(3).unwrap() == "--ii";
            let result = index.pull(
                include_configs,
                include_prompts,
                ii,
                quiet,
            ).await?;

            match result {
                PullResult::AlreadyUpToDate => {
                    println!("Already up to date.");
                },
                _ => {},
            }
        },
        Some("push") => {
            let parsed_args = ArgParser::new()
                .optional_arg_flag("--remote", ArgType::String)
                .flag_with_default(&["--no-configs", "--configs"])
                .flag_with_default(&["--no-prompts", "--prompts"])
                .flag_with_default(&["--no-queries", "--queries"])
                .optional_flag(&["--quiet"])
                .short_flag(&["--quiet"])
                .parse(&args, 2)?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/push.txt"));
                return Ok(());
            }

            let index = Index::load(root_dir?, LoadMode::QuickCheck)?;
            let remote = parsed_args.arg_flags.get("--remote").map(|s| s.to_string());
            let include_configs = parsed_args.get_flag(0).unwrap() == "--configs";
            let include_prompts = parsed_args.get_flag(1).unwrap() == "--prompts";
            let include_queries = parsed_args.get_flag(2).unwrap() == "--queries";
            let quiet = parsed_args.get_flag(3).is_some();
            let result = index.push(
                remote,
                include_configs,
                include_prompts,
                include_queries,
                quiet,
            ).await?;

            match result {
                PushResult::AlreadyUpToDate => {
                    println!("Everything up-to-date");
                },
                _ => {},
            }
        },
        Some("query") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--interactive", "--multi-turn"])
                .optional_flag(&["--json"])
                .optional_flag(&["--enable-ii", "--disable-ii"])
                .optional_flag(&["--enable-rag", "--disable-rag"])
                .optional_flag(&["--super-rerank", "--no-super-rerank"])
                .optional_arg_flag("--model", ArgType::String)
                .optional_arg_flag("--max-summaries", ArgType::uinteger())
                .optional_arg_flag("--max-retrieval", ArgType::uinteger())
                .optional_arg_flag("--schema", ArgType::String)    // pdl schema
                .optional_arg_flag("--continue", ArgType::String)  // uid
                .optional_flag(&["--agent"])
                .short_flag(&["--interactive", "--json"])
                .args(ArgType::String, ArgCount::Any)  // query
                .parse(&args, 2)?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/query.txt"));
                return Ok(());
            }

            let mut index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            let schema = match parsed_args.arg_flags.get("--schema") {
                Some(schema) => {
                    Some(parse_schema(schema)?)
                },
                None => None,
            };

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

            if let Some(model) = parsed_args.arg_flags.get("--model") {
                index.api_config.model = model.to_string();
            }

            let interactive_mode = parsed_args.get_flag(0).is_some();
            let json_mode = parsed_args.get_flag(1).is_some();
            let agent_mode = parsed_args.get_flag(5).is_some();
            let mut chat_history = if let Some(query_uid) = parsed_args.arg_flags.get("--continue") {
                let queries = index.uid_query(
                    &[query_uid.to_string()],
                    UidQueryConfig::new().query_history_only(),
                )?.get_query_histories();

                match queries.len() {
                    0 => {
                        return Err(Error::UidQueryError(format!("There's no query history that matches `{query_uid}`")));
                    },
                    1 => {
                        index.get_query_schema(queries[0])?
                    },
                    _ => {
                        return Err(Error::UidQueryError(format!("There're multiple query histories that match `{query_uid}`")));
                    },
                }
            } else {
                vec![]
            };

            match (agent_mode, interactive_mode, json_mode, &schema) {
                // TODO: multiturn agent mode
                (true, false, json_mode, schema) => {
                    if !chat_history.is_empty() {
                        return Err(Error::CliError {
                            message: String::from("You cannot continue conversation in an agent mode."),
                            span: None,
                        });
                    }

                    let query = parsed_args.get_args_exact(1)?[0].to_string();
                    let response = index.agent(
                        &query,
                        String::from("There's no information yet."),
                        AgentAction::all_actions(),
                        schema.clone(),
                        false,  // don't hide_summary
                    ).await?;
                    let turn = QueryTurn::from_agent_response(&index, &query, &response)?;
                    index.log_query_history(&[turn.clone()])?;

                    if json_mode {
                        println!("{}", serde_json::to_string_pretty(&response)?);
                    }

                    else {
                        println!("{}", turn.response.render_with_source());
                    }
                },
                (true, true, _, _) => {
                    return Err(Error::CliError {
                        message: String::from("You cannot query interactively in an agent mode."),
                        span: None,
                    });
                },
                (_, true, _, Some(_)) => {
                    return Err(Error::CliError {
                        message: String::from("You cannot set schema in an interactive mode."),
                        span: None,
                    });
                },
                (_, true, true, _) => {
                    return Err(Error::CliError {
                        message: String::from("You cannot query interactively in a json mode."),
                        span: None,
                    });
                },
                (_, true, _, _) => {
                    for turn in chat_history.iter() {
                        println!(">>> {}", turn.query);
                        println!("");
                        println!("{}", turn.response.render_with_source());
                        println!("");
                    }

                    loop {
                        let mut curr_input = String::new();
                        print!(">>> ");
                        std::io::stdout().flush()?;
                        std::io::stdin().read_line(&mut curr_input)?;
                        println!("");
                        let response = index.query(
                            &curr_input,
                            chat_history.clone(),
                            None,  // schema
                        ).await?;

                        println!("{}", response.render_with_source());
                        println!("");
                        chat_history.push(QueryTurn::new(&curr_input, &response));
                        index.log_query_history(&chat_history)?;
                    }
                },
                _ => {
                    let query = parsed_args.get_args_exact(1)?[0].to_string();
                    let response = index.query(
                        &query,
                        chat_history.clone(),
                        schema.clone(),
                    ).await?;
                    let turn = QueryTurn::new(&query, &response);
                    chat_history.push(turn);
                    index.log_query_history(&chat_history)?;

                    if json_mode {
                        println!("{}", serde_json::to_string_pretty(&response.prettify()?)?);
                    }

                    else if schema.is_some() {
                        println!("{}", response.response);
                    }

                    else {
                        println!("{}", response.render_with_source());
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
                .alias("--cached", "--staged")
                .args(ArgType::String, ArgCount::Any)  // path
                .parse(&args, 2)?;

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
            let staged = parsed_args.get_flag(4).unwrap_or(String::from("--staged")) == "--staged";
            let processed = parsed_args.get_flag(4).unwrap_or(String::from("--processed")) == "--processed";
            let mut files = parsed_args.get_args();
            let mut result = RemoveResult::default();

            if all {
                if !files.is_empty() {
                    return Err(Error::CliError {
                        message: String::from("You cannot use `--all` options with paths."),
                        span: None,
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
                    span: Span::End.render(&args, 2),
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
                .optional_arg_flag("--max-retrieval", ArgType::uinteger())
                .optional_arg_flag("--max-summaries", ArgType::uinteger())
                .arg_flag_with_default("--abbrev", "9", ArgType::integer_between(Some(4), Some(64)))
                .short_flag(&["--json"])
                .args(ArgType::String, ArgCount::Exact(1))  // query
                .parse(&args, 2)?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/retrieve-chunks.txt"));
                return Ok(());
            }

            let uid_only = parsed_args.get_flag(0).is_some();
            let json_mode = parsed_args.get_flag(1).is_some();
            let rerank = parsed_args.get_flag(2).unwrap() == "--rerank";
            let abbrev = parsed_args.arg_flags.get("--abbrev").unwrap().parse::<usize>().unwrap();
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
                                |chunk| chunk.uid.abbrev(abbrev)
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
                                    (String::from("uid"), chunk.uid.abbrev(abbrev).into()),
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
                    println!("uid: {}", chunk.uid.abbrev(abbrev));
                    println!("source: {}", chunk.render_source());
                    println!("title: {}", chunk.title);
                    println!("summary: {}", chunk.summary);
                }
            }
        },
        Some("status") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--json"])
                .short_flag(&["--json"])
                .parse(&args, 2)?;

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
        Some("summary") => {
            let parsed_args = ArgParser::new().parse(&args, 2);

            // `ArgParser.parse()` fails unless it's `rag summary --help`
            match parsed_args {
                Ok(parsed_args) if parsed_args.show_help() => {
                    println!("{}", include_str!("../docs/commands/summary.txt"));
                    return Ok(());
                },
                _ => {},
            }

            let mut index = Index::load(root_dir?, LoadMode::QuickCheck)?;

            match args.get(2).map(|s| s.as_str()) {
                Some("--remove") => {
                    ArgParser::new()
                        .args(ArgType::String, ArgCount::None)
                        .parse(&args, 3)?;

                    index.remove_summary()?;
                },
                Some("--set") => {
                    let parsed_args = ArgParser::new()
                        .args(ArgType::String, ArgCount::Exact(1))
                        .parse(&args, 3)?;

                    index.set_summary(&parsed_args.get_args_exact(1)?[0])?;
                },
                _ => {
                    let parsed_args = ArgParser::new()
                        .optional_flag(&["--force", "--cached"])
                        .short_flag(&["--force"])
                        .args(ArgType::String, ArgCount::None)
                        .parse(&args, 2)?;
                    let summary_mode = parsed_args.get_flag(0).map(|flag| SummaryMode::parse_flag(&flag)).unwrap_or(None);

                    match index.summary(summary_mode).await? {
                        Some(s) => { println!("{s}"); },
                        None => {  // `--cached`, but no summary
                            return Err(Error::NoSummary);
                        },
                    }
                },
            }
        },
        Some("tfidf") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--uid-only"])
                .optional_flag(&["--json"])
                .flag_with_default(&["--keyword", "--query"])
                .arg_flag_with_default("--limit", "10", ArgType::uinteger())
                .arg_flag_with_default("--abbrev", "9", ArgType::integer_between(Some(4), Some(64)))
                .short_flag(&["--json"])
                .args(ArgType::String, ArgCount::Exact(1))
                .parse(&args, 2)?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/tfidf.txt"));
                return Ok(());
            }

            let uid_only = parsed_args.get_flag(0).is_some();
            let json_mode = parsed_args.get_flag(1).is_some();
            let query_mode = parsed_args.get_flag(2).unwrap_or(String::new()) == "--query";
            let abbrev = parsed_args.arg_flags.get("--abbrev").unwrap().parse::<usize>().unwrap();

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
                                |chunk| chunk.uid.abbrev(abbrev)
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
                                    (String::from("uid"), chunk.uid.abbrev(abbrev).into()),
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
                        println!("{}", chunk.uid.abbrev(abbrev));
                        continue;
                    }

                    println!("--------------------------");
                    println!("score: {}", tfidf.score);
                    println!("uid: {}", chunk.uid.abbrev(abbrev));
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
        Some("uid") => {
            let parsed_args = ArgParser::new()
                .arg_flag_with_default("--abbrev", "64", ArgType::integer_between(Some(4), Some(64)))
                .parse(&args, 2)?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/uid.txt"));
                return Ok(());
            }

            let abbrev = parsed_args.arg_flags.get("--abbrev").unwrap().parse::<usize>().unwrap();
            let mut index = Index::load(root_dir?, LoadMode::OnlyJson)?;
            println!("{}", index.calculate_and_save_uid()?.abbrev(abbrev));
        },
        Some("version") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--build-options"])
                .optional_flag(&["--json"])
                .short_flag(&["--json"])
                .parse(&args, 2)?;

            if parsed_args.show_help() {
                println!("{}", include_str!("../docs/commands/version.txt"));
                return Ok(());
            }

            let build_options = parsed_args.get_flag(0).is_some();
            let json_mode = parsed_args.get_flag(1).is_some();

            if build_options {
                let build_options = get_build_options();

                if json_mode {
                    println!("{}", serde_json::to_string_pretty(&build_options)?);
                }

                else {
                    println!("version: {}", build_options.version);
                    println!("profile: {}", build_options.profile);
                    println!("features:");

                    for (feature, enabled) in build_options.features.iter() {
                        println!("    {feature}: {}", if *enabled { "enabled" } else { "disabled" });
                    }
                }
            }

            else {
                let s = format!("ragit {}", ragit::VERSION);

                if json_mode {
                    println!("{s:?}");
                }

                else {
                    println!("{s}");
                }
            }
        },
        Some(invalid_command) => {
            // TODO: this is a very bad idea. I need a way to programatically load the list
            // of the commands.
            let similar_command = get_closest_string(
                &[
                    "add",
                    "archive-create", "create-archive", "archive",
                    "archive-extract", "extract-archive", "extract",
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
                    "ls-queries",
                    "ls-terms",
                    "merge",
                    "meta",
                    "migrate",
                    "model",
                    "pdl",
                    "pull",
                    "push",
                    "query",
                    "remove", "rm",
                    "retrieve-chunks",
                    "status",
                    "summary",
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
                span: Span::NthArg(0).render(&args, 1),
            });
        },
        None => {
            return Err(Error::CliError {
                message: String::from("Command is missing. Run `rag help` to get help."),
                span: Span::End.render(&args, 2),
            });
        },
    }

    Ok(())
}

// it starts from "." and goes up until it finds ".ragit"
// you can run git commands anywhere inside a repo, and I want ragit to be like that
fn find_root() -> Result<String, Error> {
    let mut curr = String::from(".");

    loop {
        let curr_files = read_dir(&curr, false)?;

        for f in curr_files.iter() {
            if basename(f)? == INDEX_DIR_NAME {
                return Ok(curr);
            }
        }

        curr = join(&curr, "..")?;
    }
}
