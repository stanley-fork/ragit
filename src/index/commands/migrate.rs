use super::Index;
use crate::error::Error;
use crate::index::{INDEX_DIR_NAME, INDEX_FILE_NAME};
use flate2::read::GzDecoder;
use json::JsonValue;
use ragit_api::{JsonType, get_type};
use ragit_fs::{
    WriteMode,
    copy_dir,
    create_dir_all,
    exists,
    extension,
    join,
    join3,
    read_bytes,
    read_dir,
    read_string,
    remove_dir_all,
    rename,
    write_string,
};
use regex::Regex;
use std::{cmp, fmt};
use std::collections::HashMap;
use std::io::Read;
use std::str::FromStr;

pub type Path = String;

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

        match json::parse(&j)? {
            JsonValue::Object(obj) => match obj.get("ragit_version") {
                Some(v) => match v.as_str() {
                    Some(v) => Ok(v.parse::<VersionInfo>()?),
                    None => Err(Error::JsonTypeError {
                        expected: JsonType::String,
                        got: get_type(v),
                    }),
                },
                None => Err(Error::BrokenIndex(String::from("`ragit_version` is not found in `index.json`."))),
            },
            v => Err(Error::JsonTypeError {
                expected: JsonType::Object,
                got: get_type(&v),
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
    /// it may successfully auto-migrate, but still is corrupted.
    /// If the original knowledge-base perfect and there's no compatibility issue but
    /// the client doesn't know that, this function may fail but there's no problem
    /// using the knowledge-base.
    pub fn auto_migrate(root_dir: &Path) -> Result<(), Error> {
        let base_version = Index::check_ragit_version(root_dir)?;
        let client_version_str = crate::VERSION.to_string();
        let client_version = client_version_str.parse::<VersionInfo>()?;

        // It's still a problem.
        // Even though the client is outdated, compatibility issue is very unlikely.
        // But the client can never tell that.
        //
        // The easiest fix is to implement auto-migration for all the versions, and tell users to always keep their ragit version up-to-date.
        // But those are not always possible.
        if base_version > client_version {
            Err(Error::CannotAutoMigrate {
                from: base_version.to_string(),
                to: client_version_str,
            })
        }

        else if base_version == client_version {
            Ok(())
        }

        else {
            // TODO: make `HashMap<(base: VersionInfo, client: VersionInfo), Fn(base, client, root_dir: &Path) -> Result<(), Error>>`
            //       for now, there's only 1 case, so it uses very naive if branches
            if base_version.major == 0 && base_version.minor < 2 {
                let index_dir = join(root_dir, INDEX_DIR_NAME)?;
                let tmp_dir = create_tmp_dir()?;
                let tmp_index_dir = join(&tmp_dir, INDEX_DIR_NAME)?;
                copy_dir(&index_dir, &tmp_index_dir)?;

                match auto_migrate_0_1_1_to_0_2_0(base_version, client_version, &tmp_dir) {
                    Ok(()) => {
                        remove_dir_all(&index_dir)?;
                        rename(&tmp_index_dir, &index_dir)?;
                        remove_dir_all(&tmp_dir)?;
                        Ok(())
                    },
                    Err(e) => {
                        remove_dir_all(&tmp_dir)?;
                        Err(e)
                    },
                }
            }

            else {
                Err(Error::CannotAutoMigrate {
                    from: base_version.to_string(),
                    to: client_version_str,
                })
            }
        }
    }
}

fn create_tmp_dir() -> Result<Path, Error> {
    // TODO: remove this random function, and remove the dependency from this crate
    let dir_name = format!("__tmp_{:x}", rand::random::<u64>());  // let's hope it doesn't conflict
    create_dir_all(&dir_name)?;
    Ok(dir_name)
}

fn auto_migrate_0_1_1_to_0_2_0(base_version: VersionInfo, client_version: VersionInfo, root_dir: &Path) -> Result<(), Error> {
    let index_at = join3(
        root_dir,
        ".ragit",
        "index.json",
    )?;
    let j = read_string(&index_at)?;
    let mut j = json::parse(&j)?;
    let file_hash_re = Regex::new(r"(\d{8})_([0-9a-f]{64})").unwrap();
    let uid_re = Regex::new(r"[0-9a-z]{64}").unwrap();
    let mut processed_files_cache;

    match &mut j {
        JsonValue::Object(ref mut index) => {
            index.insert("ragit_version", "0.2.0".into());

            if index.remove("chunk_files").is_none() {
                return Err(Error::BrokenIndex(String::from("`index.json` is missing `chunk_files` field.")));
            }

            match index.get_mut("processed_files") {
                Some(JsonValue::Object(ref mut processed_files)) => {
                    processed_files_cache = HashMap::with_capacity(processed_files.len());

                    for (file_name, file_hash) in processed_files.clone().iter() {
                        match file_hash.as_str() {
                            Some(file_hash) => match file_hash_re.captures(file_hash) {
                                Some(file_hash_cap) => {
                                    let file_hash = format!("{}0{}", &file_hash_cap[2], &file_hash_cap[1]);
                                    processed_files.insert(file_name, file_hash.clone().into());
                                    processed_files_cache.insert(file_name.to_string(), file_hash);
                                },
                                None => {
                                    return Err(Error::BrokenIndex(format!("`index.json` has a corrupted file hash: `{file_hash}`.")));
                                },
                            },
                            None => {
                                return Err(Error::JsonTypeError {
                                    expected: JsonType::String,
                                    got: get_type(file_hash),
                                });
                            },
                        }
                    }
                },
                Some(v) => {
                    return Err(Error::JsonTypeError {
                        expected: JsonType::Object,
                        got: get_type(v),
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
                got: get_type(&j),
            });
        },
    }

    write_string(
        &index_at,
        &j.pretty(4),
        WriteMode::CreateOrTruncate,
    )?;
    remove_dir_all(
        &join3(
            root_dir,
            ".ragit",
            "chunk_index",
        )?,
    )?;

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
    )?)? {
        if extension(&chunk_file)?.unwrap_or(String::new()) != "chunks" {
            continue;
        }

        let chunks = load_chunks_0_1_1(&chunk_file)?;

        match chunks {
            JsonValue::Array(mut chunks) => {
                for chunk in chunks.iter_mut() {
                    match chunk {
                        JsonValue::Object(ref mut obj) => {
                            let file_name = match obj.get("file") {
                                Some(file_name) => match file_name.as_str() {
                                    Some(file_name) => file_name.to_string(),
                                    None => {
                                        return Err(Error::JsonTypeError {
                                            expected: JsonType::String,
                                            got: get_type(file_name),
                                        });
                                    },
                                },
                                None => {
                                    return Err(Error::BrokenIndex(String::from("A corrupted chunk.")));
                                },
                            };
                            let file_index = match obj.get("index") {
                                Some(index) => match index.as_usize() {
                                    Some(index) => index,
                                    None => {
                                        return Err(Error::JsonTypeError {
                                            expected: JsonType::Usize,
                                            got: get_type(index),
                                        });
                                    },
                                },
                                None => {
                                    return Err(Error::BrokenIndex(String::from("A corrupted chunk.")));
                                },
                            };

                            // 0.1.1 uses 1-based index
                            match obj.get_mut("index") {
                                Some(ref mut index) => match index.as_usize() {
                                    Some(n) => {
                                        **index = JsonValue::from(n - 1);
                                    },
                                    None => {
                                        return Err(Error::JsonTypeError {
                                            expected: JsonType::Usize,
                                            got: get_type(index),
                                        });
                                    },
                                },
                                None => {
                                    return Err(Error::BrokenIndex(String::from("A corrupted chunk.")));
                                },
                            }

                            match obj.get("uid") {
                                Some(uid) => match uid.as_str() {
                                    Some(uid) if uid_re.is_match(uid) => {
                                        let chunk_at = join(
                                            &tmp_chunk_dir,
                                            uid.get(0..2).unwrap(),
                                        )?;

                                        if !exists(&chunk_at) {
                                            create_dir_all(&chunk_at)?;
                                        }

                                        match file_to_chunks_map.get_mut(&file_name) {
                                            Some(uids) => {
                                                uids.push((uid.to_string(), file_index));
                                            },
                                            None => {
                                                file_to_chunks_map.insert(file_name, vec![(uid.to_string(), file_index)]);
                                            },
                                        }

                                        // TODO: respect build_config.compress_threshold
                                        write_string(
                                            &join(&chunk_at, &format!("{}.chunk", uid.get(2..).unwrap()))?,
                                            &format!("\n{}", chunk.pretty(4)),  // chunk prefix for an un-compressed chunk
                                            WriteMode::AlwaysCreate,
                                        )?;
                                    },
                                    Some(uid) => {
                                        return Err(Error::BrokenIndex(format!("There's a malformed uid: `{uid}`.")));
                                    },
                                    None => {
                                        return Err(Error::JsonTypeError {
                                            expected: JsonType::String,
                                            got: get_type(uid),
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
                                expected: JsonType::Array,
                                got: get_type(chunk),
                            });
                        },
                    }
                }
            },
            _ => {
                return Err(Error::JsonTypeError {
                    expected: JsonType::Array,
                    got: get_type(&chunks),
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
        let file_hash = match processed_files_cache.get(&file_name) {
            Some(file_hash) => file_hash.to_string(),
            None => {
                return Err(Error::BrokenIndex(format!("File hash not found: `{file_name}`")));
            },
        };
        let index_path = join(&file_index_at, file_hash.get(0..2).unwrap())?;

        if !exists(&index_path) {
            create_dir_all(&index_path)?;
        }

        write_string(
            &join(&index_path, file_hash.get(2..).unwrap())?,
            &chunks.into_iter().map(|(uid, _)| uid).collect::<Vec<_>>().join("\n"),
            WriteMode::AlwaysCreate,
        )?;
    }

    Ok(())
}

fn load_chunks_0_1_1(path: &str) -> Result<JsonValue, Error> {
    let content = read_bytes(path)?;

    match content.get(0) {
        Some(b) if *b == b'c' => {
            let mut decompressed = vec![];
            let mut gz = GzDecoder::new(&content[1..]);
            gz.read_to_end(&mut decompressed)?;

            Ok(json::parse(&String::from_utf8_lossy(&decompressed).to_string())?)
        },
        Some(b) if *b == b'\n' => Ok(json::parse(&String::from_utf8_lossy(&content[1..]).to_string())?),
        Some(b) => Err(Error::BrokenIndex(format!("Unknown chunk prefix: {}", *b as char))),
        None => Err(Error::BrokenIndex(format!("An empty chunk file."))),
    }
}

// This is an internal representation of versions. I don't think it's the best
// way to manage versions. There must be better ways and I need more research on those.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
struct VersionInfo {
    major: u16,
    minor: u16,
    patch: u16,
    dev: bool,
}

impl FromStr for VersionInfo {
    type Err = Error;

    fn from_str(s: &str) -> Result<VersionInfo, Error> {
        let version_re = Regex::new(r"(\d{0,4})\.(\d{0,4})\.(\d{0,4})(?:-([a-zA-Z0-9]+))?").unwrap();

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
