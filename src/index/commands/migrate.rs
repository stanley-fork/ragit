use super::Index;
use crate::error::Error;
use crate::index::{INDEX_DIR_NAME, INDEX_FILE_NAME};
use crate::prompts::PROMPTS;
use flate2::read::GzDecoder;
use lazy_static::lazy_static;
use ragit_api::JsonType;
use ragit_fs::{
    WriteMode,
    copy_dir,
    copy_file,
    create_dir_all,
    exists,
    extension,
    file_name,
    file_size,
    join,
    join3,
    join4,
    read_bytes,
    read_dir,
    read_string,
    remove_dir_all,
    remove_file,
    rename,
    set_extension,
    write_bytes,
    write_string,
};
use regex::Regex;
use serde_json::{Number, Value};
use std::{cmp, fmt};
use std::collections::HashMap;
use std::io::Read;
use std::str::FromStr;

pub type Path = String;

lazy_static! {
    static ref FILE_UID_RE: Regex = Regex::new(r"^(\d{8})_([0-9a-f]{48})[0-9a-f]{16}$").unwrap();
    static ref UID_RE: Regex = Regex::new(r"[0-9a-z]{64}").unwrap();
    static ref VERSION_RE: Regex = Regex::new(r"(\d{0,4})\.(\d{0,4})\.(\d{0,4})(?:-([a-zA-Z0-9]+))?").unwrap();
}

impl Index {
    /// It reads version info at `root/.ragit/index.json`. Make sure that the
    /// file exists and `index.json` has `"ragit_version" field.`
    pub fn check_ragit_version(root_dir: &Path) -> Result<VersionInfo, Error> {
        let index_at = join3(
            root_dir,
            INDEX_DIR_NAME,
            INDEX_FILE_NAME,
        )?;
        let j = read_string(&index_at)?;

        match serde_json::from_str::<Value>(&j)? {
            Value::Object(obj) => match obj.get("ragit_version") {
                Some(v) => match v.as_str() {
                    Some(v) => Ok(v.parse::<VersionInfo>()?),
                    None => Err(Error::JsonTypeError {
                        expected: JsonType::String,
                        got: v.into(),
                    }),
                },
                None => Err(Error::BrokenIndex(String::from("`ragit_version` is not found in `index.json`."))),
            },
            v => Err(Error::JsonTypeError {
                expected: JsonType::Object,
                got: (&v).into(),
            }),
        }
    }

    /// You can auto-migrate knowledge-bases built by older versions of
    /// ragit. It doesn't modify the contents of the knowledge-bases, but
    /// may change structures or formats of the files.
    ///
    /// If it returns `Ok(())`, you can access the migrated knowledge-base
    /// by `Index::load(root_dir)`.
    ///
    /// The result of this function doesn't always mean whether the knowledge-base
    /// is corrupted or not. For example, if the original knowledge-base is corrupted,
    /// it may successfully migrate, but still is corrupted.
    /// If the original knowledge-base perfect and there's no compatibility issue but
    /// the client doesn't know that, this function may fail but there's no problem
    /// using the knowledge-base.
    ///
    /// It assumes that `recover` is run after `migrate`. It doesn't do any operation
    /// that can be handled by `recover`.
    pub fn migrate(root_dir: &Path) -> Result<Option<(VersionInfo, VersionInfo)>, Error> {
        let base_version = Index::check_ragit_version(root_dir)?;
        let client_version_str = crate::VERSION.to_string();
        let client_version = client_version_str.parse::<VersionInfo>()?;

        // It's still a problem.
        // Even though the client is outdated, compatibility issue is very unlikely.
        // But the client can never tell that.
        //
        // The easiest fix is to implement migration for all the versions, and tell users to always keep their ragit version up-to-date.
        // But those are not always possible.
        if base_version > client_version {
            Err(Error::CannotMigrate {
                from: base_version.to_string(),
                to: client_version_str,
            })
        }

        else if base_version == client_version {
            Ok(None)
        }

        else {
            if base_version.major == 0 && base_version.minor < 2 {
                let index_dir = join(root_dir, INDEX_DIR_NAME)?;
                let tmp_dir = create_tmp_dir()?;
                let tmp_index_dir = join(&tmp_dir, INDEX_DIR_NAME)?;
                copy_dir(&index_dir, &tmp_index_dir)?;

                match migrate_0_1_1_to_0_2_x(&tmp_dir) {
                    Ok(()) => {
                        remove_dir_all(&index_dir)?;
                        rename(&tmp_index_dir, &index_dir)?;
                        remove_dir_all(&tmp_dir)?;
                        Ok(Some((base_version, client_version)))
                    },
                    Err(e) => {
                        remove_dir_all(&tmp_dir)?;
                        Err(e)
                    },
                }
            }

            // as of v0.2.1, there's no compatibility issue in v0.2.x
            else {
                let index_dir = join(root_dir, INDEX_DIR_NAME)?;
                let tmp_dir = create_tmp_dir()?;
                let tmp_index_dir = join(&tmp_dir, INDEX_DIR_NAME)?;
                copy_dir(&index_dir, &tmp_index_dir)?;

                match update_version_string(&tmp_dir, crate::VERSION) {
                    Ok(()) => {
                        remove_dir_all(&index_dir)?;
                        rename(&tmp_index_dir, &index_dir)?;
                        remove_dir_all(&tmp_dir)?;
                        Ok(Some((base_version, client_version)))
                    },
                    Err(e) => {
                        remove_dir_all(&tmp_dir)?;
                        Err(e)
                    },
                }
            }
        }
    }
}

fn create_tmp_dir() -> Result<Path, Error> {
    let mut dir_name = String::new();

    for i in 0..1000 {
        dir_name = format!("__tmp_{i:03}");

        if !exists(&dir_name) {
            break;
        }
    }

    create_dir_all(&dir_name)?;
    Ok(dir_name)
}

fn update_version_string(root_dir: &Path, new_version: &str) -> Result<(), Error> {
    let index_at = join3(
        root_dir,
        ".ragit",
        "index.json",
    )?;
    let j = read_string(&index_at)?;
    let mut j = serde_json::from_str::<Value>(&j)?;

    match &mut j {
        Value::Object(ref mut index) => {
            index.insert(String::from("ragit_version"), new_version.into());
        },
        _ => {
            return Err(Error::JsonTypeError {
                expected: JsonType::Object,
                got: (&j).into(),
            });
        },
    }

    write_bytes(
        &index_at,
        &serde_json::to_vec_pretty(&j)?,
        WriteMode::CreateOrTruncate,
    )?;

    Ok(())
}

fn migrate_0_1_1_to_0_2_x(root_dir: &Path) -> Result<(), Error> {
    let index_at = join3(
        root_dir,
        ".ragit",
        "index.json",
    )?;
    let j = read_string(&index_at)?;
    let mut j = serde_json::from_str::<Value>(&j)?;
    let file_uid_re = &FILE_UID_RE;
    let uid_re = &UID_RE;
    let mut processed_files_cache;
    let mut image_uid_map = HashMap::new();

    match &mut j {
        Value::Object(ref mut index) => {
            index.insert(String::from("ragit_version"), "0.2.0".into());
            index.insert(String::from("ii_status"), Value::Object(vec![(String::from("type"), String::from("None").into())].into_iter().collect()));

            if index.remove("chunk_files").is_none() {
                return Err(Error::BrokenIndex(String::from("`index.json` is missing `chunk_files` field.")));
            }

            match index.get_mut("processed_files") {
                Some(Value::Object(ref mut processed_files)) => {
                    processed_files_cache = HashMap::with_capacity(processed_files.len());

                    for (file_name, file_uid) in processed_files.clone().iter() {
                        match file_uid.as_str() {
                            Some(file_uid) => match file_uid_re.captures(file_uid) {
                                Some(file_uid_cap) => {
                                    let file_uid = format!("{}00000003{:08x}", &file_uid_cap[2], file_uid_cap[1].parse::<usize>().unwrap());
                                    processed_files.insert(
                                        file_name.to_string(),
                                        vec![
                                            (String::from("high"), Value::Number(Number::from_u128(u128::from_str_radix(file_uid.get(0..32).unwrap(), 16).unwrap()).unwrap())),
                                            (String::from("low"), Value::Number(Number::from_u128(u128::from_str_radix(file_uid.get(32..).unwrap(), 16).unwrap()).unwrap())),
                                        ].into_iter().collect(),
                                    );
                                    processed_files_cache.insert(file_name.to_string(), file_uid);
                                },
                                None => {
                                    return Err(Error::BrokenIndex(format!("`index.json` has a corrupted file uid: `{file_uid}`.")));
                                },
                            },
                            None => {
                                return Err(Error::JsonTypeError {
                                    expected: JsonType::String,
                                    got: file_uid.into(),
                                });
                            },
                        }
                    }
                },
                Some(v) => {
                    return Err(Error::JsonTypeError {
                        expected: JsonType::Object,
                        got: (&*v).into(),
                    });
                },
                None => {
                    return Err(Error::BrokenIndex(String::from("`index.json` is missing `processed_files` field.")));
                },
            }
        },
        _ => {
            return Err(Error::JsonTypeError {
                expected: JsonType::Object,
                got: (&j).into(),
            });
        },
    }

    write_bytes(
        &index_at,
        &serde_json::to_vec_pretty(&j)?,
        WriteMode::CreateOrTruncate,
    )?;
    remove_dir_all(
        &join3(
            root_dir,
            ".ragit",
            "chunk_index",
        )?,
    )?;

    let image_dir = join3(
        root_dir,
        ".ragit",
        "images",
    )?;
    let mut image_size_map = HashMap::new();

    for image_file in read_dir(&image_dir, false)? {
        let curr_ext = extension(&image_file)?.unwrap_or(String::new());

        if curr_ext != "png" && curr_ext != "json" {
            continue;
        }

        let uid = file_name(&image_file)?;
        let image_size = match image_size_map.get(&uid) {
            Some(n) => *n,
            None => {
                let s = file_size(&set_extension(&image_file, "png")?)?;
                image_size_map.insert(uid.to_string(), s);
                s
            },
        };
        let new_uid = update_uid_schema(&uid, 2, image_size);
        image_uid_map.insert(uid.clone(), new_uid.clone());

        let image_at = join(
            &image_dir,
            &new_uid.get(0..2).unwrap(),
        )?;

        if !exists(&image_at) {
            create_dir_all(&image_at)?;
        }

        copy_file(
            &image_file,
            &join(
                &image_at,
                &set_extension(
                    &new_uid.get(2..).unwrap(),
                    &curr_ext,
                )?,
            )?,
        )?;
        remove_file(&image_file)?;
    }

    let tmp_chunk_dir = join3(
        root_dir,
        ".ragit",
        "chunks-tmp",
    )?;
    let mut file_to_chunks_map: HashMap<String, Vec<(String, usize)>> = HashMap::new();

    for chunk_file in read_dir(&join3(
        root_dir,
        ".ragit",
        "chunks",
    )?, false)? {
        if extension(&chunk_file)?.unwrap_or(String::new()) != "chunks" {
            continue;
        }

        let chunks = load_chunks_0_1_1(&chunk_file)?;

        match chunks {
            Value::Array(mut chunks) => {
                for chunk in chunks.iter_mut() {
                    match chunk {
                        Value::Object(ref mut obj) => {
                            match obj.get("images") {
                                Some(Value::Array(images)) if images.len() > 0 => {
                                    let mut new_images: Vec<Value> = Vec::with_capacity(images.len());

                                    for image in images.iter() {
                                        match image {
                                            Value::String(uid) => {
                                                let new_uid = match image_uid_map.get(uid) {
                                                    Some(new_uid) => new_uid.to_string(),
                                                    _ => {
                                                        return Err(Error::BrokenIndex(format!(
                                                            "chunk `{}` is pointing to an image `{}`, which does not exist",
                                                            obj.get("uid").map(|s| s.as_str().map(|s| s.to_string()).unwrap_or(String::from("Unknown"))).unwrap_or(String::from("Unknown")),
                                                            uid,
                                                        )));
                                                    },
                                                };

                                                new_images.push(vec![
                                                    (String::from("high"), Value::Number(Number::from_u128(u128::from_str_radix(new_uid.get(0..32).unwrap(), 16).unwrap()).unwrap())),
                                                    (String::from("low"), Value::Number(Number::from_u128(u128::from_str_radix(new_uid.get(32..).unwrap(), 16).unwrap()).unwrap())),
                                                ].into_iter().collect());
                                            },
                                            v => {
                                                return Err(Error::JsonTypeError {
                                                    expected: JsonType::String,
                                                    got: v.into(),
                                                });
                                            },
                                        }
                                    }

                                    obj.insert(String::from("images"), new_images.into());
                                },
                                _ => {},
                            }

                            let file_name = match obj.get("file") {
                                Some(file_name) => match file_name.as_str() {
                                    Some(file_name) => file_name.to_string(),
                                    None => {
                                        return Err(Error::JsonTypeError {
                                            expected: JsonType::String,
                                            got: file_name.into(),
                                        });
                                    },
                                },
                                None => {
                                    return Err(Error::BrokenIndex(String::from("A corrupted chunk.")));
                                },
                            };
                            let file_index = match obj.get("index") {
                                Some(index) => match index.as_u64() {
                                    Some(index) => index as usize,
                                    None => {
                                        return Err(Error::JsonTypeError {
                                            expected: JsonType::Usize,
                                            got: index.into(),
                                        });
                                    },
                                },
                                None => {
                                    return Err(Error::BrokenIndex(String::from("A corrupted chunk.")));
                                },
                            };
                            obj.remove("file");
                            obj.remove("index");
                            obj.insert(String::from("source"), Value::Object(vec![
                                (String::from("type"), String::from("File").into()),
                                (String::from("path"), file_name.clone().into()),

                                // ragit 0.1.1 uses 1-base index
                                (String::from("index"), (file_index - 1).into()),
                            ].into_iter().collect()));
                            obj.insert(String::from("searchable"), true.into());
                            let data_len = match obj.get("data") {
                                Some(Value::String(d)) => d.len(),
                                Some(d) => {
                                    return Err(Error::JsonTypeError {
                                        expected: JsonType::String,
                                        got: d.into(),
                                    });
                                },
                                None => {
                                    return Err(Error::BrokenIndex(format!(
                                        "chunk `{}` does not have `data` field.",
                                        obj.get("uid").map(|s| s.as_str().map(|s| s.to_string()).unwrap_or(String::from("Unknown"))).unwrap_or(String::from("Unknown")),
                                    )));
                                },
                            };

                            match obj.get("uid") {
                                Some(uid) => match uid.as_str() {
                                    Some(uid) if uid_re.is_match(uid) => {
                                        let uid = uid.to_string();
                                        let new_uid = update_uid_schema(&uid, 1, data_len as u64);
                                        obj.insert(
                                            String::from("uid"),
                                            vec![
                                                (String::from("high"), Value::Number(Number::from_u128(u128::from_str_radix(new_uid.get(0..32).unwrap(), 16).unwrap()).unwrap())),
                                                (String::from("low"), Value::Number(Number::from_u128(u128::from_str_radix(new_uid.get(32..).unwrap(), 16).unwrap()).unwrap())),
                                            ].into_iter().collect(),
                                        );
                                        let chunk_at = join(
                                            &tmp_chunk_dir,
                                            new_uid.get(0..2).unwrap(),
                                        )?;

                                        if !exists(&chunk_at) {
                                            create_dir_all(&chunk_at)?;
                                        }

                                        match file_to_chunks_map.get_mut(&file_name) {
                                            Some(uids) => {
                                                uids.push((new_uid.to_string(), file_index));
                                            },
                                            None => {
                                                file_to_chunks_map.insert(file_name, vec![(new_uid.to_string(), file_index)]);
                                            },
                                        }

                                        // TODO: respect build_config.compress_threshold
                                        write_bytes(
                                            &join(&chunk_at, &format!("{}.chunk", new_uid.get(2..).unwrap()))?,
                                            &vec![
                                                vec![b'\n'],  // chunk prefix for an un-compressed chunk
                                                serde_json::to_vec_pretty(&chunk)?,
                                            ].concat(),
                                            WriteMode::AlwaysCreate,
                                        )?;
                                    },
                                    Some(uid) => {
                                        return Err(Error::BrokenIndex(format!("There's a malformed uid: `{uid}`.")));
                                    },
                                    None => {
                                        return Err(Error::JsonTypeError {
                                            expected: JsonType::String,
                                            got: uid.into(),
                                        });
                                    },
                                },
                                None => {
                                    return Err(Error::BrokenIndex(String::from("There's a chunk without uid.")));
                                },
                            }
                        },
                        _ => {
                            return Err(Error::JsonTypeError {
                                expected: JsonType::Object,
                                got: (&*chunk).into(),
                            });
                        },
                    }
                }
            },
            _ => {
                return Err(Error::JsonTypeError {
                    expected: JsonType::Array,
                    got: (&chunks).into(),
                });
            },
        }
    }

    remove_dir_all(&join3(
        root_dir,
        ".ragit",
        "chunks",
    )?)?;
    rename(
        &join3(
            root_dir,
            ".ragit",
            "chunks-tmp",
        )?, &join3(
            root_dir,
            ".ragit",
            "chunks",
        )?,
    )?;

    let file_index_at = join3(
        root_dir,
        ".ragit",
        "files",
    )?;

    for (file_name, mut chunks) in file_to_chunks_map.into_iter() {
        chunks.sort_by_key(|(_, index)| *index);
        let file_uid = match processed_files_cache.get(&file_name) {
            Some(file_uid) => file_uid.to_string(),
            None => {
                return Err(Error::BrokenIndex(format!("File hash not found: `{file_name}`")));
            },
        };
        let index_path = join(&file_index_at, file_uid.get(0..2).unwrap())?;

        if !exists(&index_path) {
            create_dir_all(&index_path)?;
        }

        write_string(
            &join(&index_path, file_uid.get(2..).unwrap())?,
            &chunks.into_iter().map(|(uid, _)| uid).collect::<Vec<_>>().join("\n"),
            WriteMode::AlwaysCreate,
        )?;
    }

    update_configs(
        &root_dir,
        vec![
            ConfigUpdate::add("query", "enable_ii", true),
            ConfigUpdate::remove("build", "chunks_per_json"),
            ConfigUpdate::update_if("api", "model", "llama3.1-70b-groq", "llama3.3-70b-groq"),
        ],
    )?;

    let prompt_path = join4(
        root_dir,
        ".ragit",
        "prompts",
        "summarize_chunks.pdl",
    )?;
    write_string(
        &prompt_path,
        PROMPTS.get("summarize_chunks").unwrap(),
        WriteMode::AlwaysCreate,
    )?;

    Ok(())
}

fn load_chunks_0_1_1(path: &str) -> Result<Value, Error> {
    let content = read_bytes(path)?;

    match content.get(0) {
        Some(b) if *b == b'c' => {
            let mut decompressed = vec![];
            let mut gz = GzDecoder::new(&content[1..]);
            gz.read_to_end(&mut decompressed)?;

            Ok(serde_json::from_slice::<Value>(&decompressed)?)
        },
        Some(b) if *b == b'\n' => Ok(serde_json::from_slice::<Value>(&content[1..])?),
        Some(b) => Err(Error::BrokenIndex(format!("Unknown chunk prefix: {}", *b as char))),
        None => Err(Error::BrokenIndex(format!("An empty chunk file."))),
    }
}

pub fn get_compatibility_warning(
    index_version: &str,
    ragit_version: &str,
) -> Option<String> {
    let index_version = match index_version.parse::<VersionInfo>() {
        Ok(v) => v,
        Err(_) => {
            return Some(format!("Unable to parse the version of the knowledge-base. It's {index_version}"));
        },
    };
    let ragit_version = match ragit_version.parse::<VersionInfo>() {
        Ok(v) => v,
        Err(_) => {
            return Some(format!("Unable to parse the current version of ragit_version. It's {ragit_version}"));
        },
    };

    if ragit_version < index_version {
        Some(format!("Ragit's version is older than the knowledge-base. Please update ragit_version."))
    }

    else if (ragit_version.major > 0 || ragit_version.minor > 1) && index_version.major == 0 && index_version.minor == 1 {
        Some(format!("The knowledge-base is outdated. Please run `rag migrate`."))
    }

    else {
        None
    }
}

// This is an internal representation of versions. I don't think it's the best
// way to manage versions. There must be better ways and I need more research on those.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct VersionInfo {
    major: u16,
    minor: u16,
    patch: u16,
    dev: bool,
}

impl FromStr for VersionInfo {
    type Err = Error;

    fn from_str(s: &str) -> Result<VersionInfo, Error> {
        let version_re = &VERSION_RE;

        match version_re.captures(s) {
            Some(cap) => {
                if let Some(m) = cap.get(4) {
                    if m.as_str() != "dev" {
                        return Err(Error::InvalidVersionString(s.to_string()));
                    }
                }

                Ok(VersionInfo {
                    major: cap[1].parse::<u16>().unwrap(),
                    minor: cap[2].parse::<u16>().unwrap(),
                    patch: cap[3].parse::<u16>().unwrap(),
                    dev: cap.get(4).map(|c| c.as_str().to_string()).unwrap_or(String::new()) == "dev",
                })
            },
            // TODO: handle custom version numbers
            None => Err(Error::InvalidVersionString(s.to_string())),
        }
    }
}

impl fmt::Display for VersionInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            fmt,
            "{}.{}.{}{}",
            self.major,
            self.minor,
            self.patch,
            if self.dev { "-dev" } else { "" },
        )
    }
}

impl PartialOrd for VersionInfo {
    fn partial_cmp(&self, other: &VersionInfo) -> Option<cmp::Ordering> {
        if self == other {
            Some(cmp::Ordering::Equal)
        }

        else {
            (
                self.major,
                self.minor,
                self.patch,
                !(self.dev as u16),  // 0.2.2-dev is lower than 0.2.2
            ).partial_cmp(&(
                other.major,
                other.minor,
                other.patch,
                !(other.dev as u16),
            ))
        }
    }
}

impl Ord for VersionInfo {
    fn cmp(&self, other: &VersionInfo) -> cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

enum ConfigUpdate {
    Add {
        file: String,
        key: String,
        value: Value,
    },
    Remove {
        file: String,
        key: String,
    },
    UpdateIf {
        file: String,
        key: String,
        pre: Value,
        post: Value,
    },
}

impl ConfigUpdate {
    pub fn add<T: Into<Value>>(file: &str, key: &str, value: T) -> Self {
        ConfigUpdate::Add {
            file: file.to_string(),
            key: key.to_string(),
            value: value.into(),
        }
    }

    pub fn remove(file: &str, key: &str) -> Self {
        ConfigUpdate::Remove {
            file: file.to_string(),
            key: key.to_string(),
        }
    }

    pub fn update_if<T: Into<Value>, U: Into<Value>>(file: &str, key: &str, pre: T, post: U) -> Self {
        ConfigUpdate::UpdateIf {
            file: file.to_string(),
            key: key.to_string(),
            pre: pre.into(),
            post: post.into(),
        }
    }

    pub fn get_file(&self) -> String {
        match self {
            ConfigUpdate::Add { file, .. } => file.to_string(),
            ConfigUpdate::Remove { file, .. } => file.to_string(),
            ConfigUpdate::UpdateIf { file, .. } => file.to_string(),
        }
    }
}

fn update_configs(root_dir: &str, updates: Vec<ConfigUpdate>) -> Result<(), Error> {
    let configs_at = join3(
        root_dir,
        ".ragit",
        "configs",
    )?;

    for update in updates.into_iter() {
        let json_at = join(
            &configs_at,
            &set_extension(
                &update.get_file(),
                "json",
            )?,
        )?;
        let j = read_string(&json_at)?;
        let mut v = serde_json::from_str::<Value>(&j)?;

        match &mut v {
            Value::Object(ref mut obj) => match update {
                ConfigUpdate::Add { key, value, .. } => {
                    obj.insert(key, value);
                },
                ConfigUpdate::Remove { key, .. } => {
                    obj.remove(&key);
                },
                ConfigUpdate::UpdateIf { key, pre, post, .. } => {
                    match obj.get(&key) {
                        Some(v) if v == &pre => {
                            obj.insert(key, post);
                        },
                        _ => {},
                    }
                },
            },
            _ => {
                return Err(Error::JsonTypeError {
                    expected: JsonType::Object,
                    got: (&v).into(),
                });
            },
        }

        write_bytes(
            &json_at,
            &serde_json::to_vec_pretty(&v)?,
            WriteMode::CreateOrTruncate,
        )?;
    }

    Ok(())
}

fn update_uid_schema(old_uid: &str, uid_type: u32, data_len: u64) -> String {
    let prefix = old_uid.get(0..48).unwrap();
    let suffix = format!("{uid_type:08x}{data_len:08x}");
    format!("{prefix}{suffix}")
}
