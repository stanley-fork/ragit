use crate::utils::get_rag_path;
use ragit_fs::{
    exists,
    extension,
    file_name,
    join,
    join3,
    join4,
    read_bytes,
    read_dir,
    read_string,
};
use serde_json::{Map, Value};
use warp::Reply;
use warp::http::StatusCode;
use warp::reply::{json, with_header, with_status};

pub fn get_index(user: String, repo: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);
    let index_path = join(&rag_path, "index.json").unwrap();

    match read_string(&index_path) {
        Ok(j) => Box::new(with_header(
            j,
            "Content-Type",
            "application/json",
        )),
        Err(_) => Box::new(with_status(
            String::new(),
            StatusCode::from_u16(404).unwrap(),
        )),
    }
}

pub fn get_config(user: String, repo: String, config: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);
    let config_path = join3(
        &rag_path,
        "configs",
        &format!("{config}.json"),
    ).unwrap();

    match read_string(&config_path) {
        Ok(j) => Box::new(with_header(
            j,
            "Content-Type",
            "application/json",
        )),
        Err(_) => Box::new(with_status(
            String::new(),
            StatusCode::from_u16(404).unwrap(),
        )),
    }
}

pub fn get_prompt(user: String, repo: String, prompt: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);
    let prompt_path = join3(
        &rag_path,
        "prompts",
        &format!("{prompt}.pdl"),
    ).unwrap();

    match read_string(&prompt_path) {
        Ok(j) => Box::new(with_header(
            j,
            "Content-Type",
            "text/plain",
        )),
        Err(_) => Box::new(with_status(
            String::new(),
            StatusCode::from_u16(404).unwrap(),
        )),
    }
}

pub fn get_chunk_count(user: String, repo: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);
    let index_path = join(&rag_path, "index.json").unwrap();

    if !exists(&index_path) {
        return Box::new(with_status(
            String::new(),
            StatusCode::from_u16(404).unwrap(),
        ));
    }

    let index_json = read_string(&index_path).unwrap_or(String::from("{}"));
    let index = serde_json::from_str::<Value>(&index_json).unwrap_or(Value::Object(Map::new()));

    match index {
        Value::Object(obj) => match obj.get("chunk_count") {
            Some(Value::Number(n)) => match n.as_u64() {
                Some(n) => Box::new(json(&n)),
                _ => Box::new(with_status(String::new(), StatusCode::from_u16(500).unwrap())),
            },
            _ => Box::new(with_status(String::new(), StatusCode::from_u16(500).unwrap())),
        },
        _ => Box::new(with_status(String::new(), StatusCode::from_u16(500).unwrap())),
    }
}

pub fn get_chunk_list(user: String, repo: String, prefix: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);
    let chunk_path = join3(
        &rag_path,
        "chunks",
        &prefix,
    ).unwrap();

    match read_dir(&chunk_path, false) {
        Ok(chunks) => Box::new(json(
            &chunks.iter().filter_map(
                |chunk| match extension(chunk) {
                    Ok(Some(e)) if e == "chunk" => file_name(chunk).ok().map(|suffix| format!("{prefix}{suffix}")),
                    _ => None,
                }
            ).collect::<Vec<String>>(),
        )),
        Err(_) => Box::new(json::<Vec<String>>(&vec![])),
    }
}

pub fn get_chunk_list_all(user: String, repo: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);
    let chunk_parents = join(
        &rag_path,
        "chunks",
    ).unwrap_or(String::new());
    let mut result = vec![];

    for prefix in 0..256 {
        let prefix = format!("{prefix:02x}");
        let chunks_at = join(
            &chunk_parents,
            &prefix,
        ).unwrap_or(String::new());

        if exists(&chunks_at) {
            for chunk in read_dir(&chunks_at, false).unwrap_or(vec![]) {
                if extension(&chunk).unwrap_or(None).unwrap_or(String::new()) == "chunk" {
                    result.push(format!("{prefix}{}", file_name(&chunk).unwrap()));
                }
            }
        }
    }

    Box::new(json(&result))
}

pub fn get_chunk(user: String, repo: String, uid: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);
    let prefix = match uid.get(0..2) {
        Some(p) => p.to_string(),
        None => {
            return Box::new(with_status(
                String::new(),
                StatusCode::from_u16(400).unwrap(),
            ));
        },
    };
    let suffix = match uid.get(2..) {
        Some(s) => s.to_string(),
        None => {
            return Box::new(with_status(
                String::new(),
                StatusCode::from_u16(400).unwrap(),
            ));
        },
    };
    let chunk_path = join4(
        &rag_path,
        "chunks",
        &prefix,
        &format!("{suffix}.chunk"),
    ).unwrap();

    match read_bytes(&chunk_path) {
        Ok(bytes) => Box::new(with_header(
            bytes,
            "Content-Type",
            "application/octet-stream",
        )),
        Err(_) => Box::new(with_status(
            String::new(),
            StatusCode::from_u16(404).unwrap(),
        )),
    }
}

pub fn get_image_list(user: String, repo: String, prefix: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);
    let image_path = join3(
        &rag_path,
        "images",
        &prefix,
    ).unwrap();

    match read_dir(&image_path, false) {
        Ok(images) => Box::new(json(
            &images.iter().filter_map(
                |image| match extension(image) {
                    Ok(Some(png)) if png == "png" => file_name(image).ok().map(|suffix| format!("{prefix}{suffix}")),
                    _ => None,
                }
            ).collect::<Vec<String>>(),
        )),
        Err(_) => Box::new(json::<Vec<String>>(&vec![])),
    }
}

pub fn get_image(user: String, repo: String, uid: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);
    let prefix = match uid.get(0..2) {
        Some(p) => p.to_string(),
        None => {
            return Box::new(with_status(
                String::new(),
                StatusCode::from_u16(400).unwrap(),
            ));
        },
    };
    let suffix = match uid.get(2..) {
        Some(s) => s.to_string(),
        None => {
            return Box::new(with_status(
                String::new(),
                StatusCode::from_u16(400).unwrap(),
            ));
        },
    };
    let image_path = join4(
        &rag_path,
        "images",
        &prefix,
        &format!("{suffix}.png"),
    ).unwrap();

    match read_bytes(&image_path) {
        Ok(bytes) => Box::new(with_header(
            bytes,
            "Content-Type",
            "image/png",
        )),
        Err(_) => Box::new(with_status(
            String::new(),
            StatusCode::from_u16(404).unwrap(),
        )),
    }
}

pub fn get_image_desc(user: String, repo: String, uid: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);
    let prefix = match uid.get(0..2) {
        Some(p) => p.to_string(),
        None => {
            return Box::new(with_status(
                String::new(),
                StatusCode::from_u16(400).unwrap(),
            ));
        },
    };
    let suffix = match uid.get(2..) {
        Some(s) => s.to_string(),
        None => {
            return Box::new(with_status(
                String::new(),
                StatusCode::from_u16(400).unwrap(),
            ));
        },
    };
    let image_path = join4(
        &rag_path,
        "images",
        &prefix,
        &format!("{suffix}.json"),
    ).unwrap();

    match read_bytes(&image_path) {
        Ok(bytes) => Box::new(with_header(
            bytes,
            "Content-Type",
            "application/json",
        )),
        Err(_) => Box::new(with_status(
            String::new(),
            StatusCode::from_u16(404).unwrap(),
        )),
    }
}

// NOTE: a `.ragit/` may or may not have `meta.json`
pub fn get_meta(user: String, repo: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);

    if !exists(&rag_path) {
        return Box::new(with_status(String::new(), StatusCode::from_u16(404).unwrap()));
    }

    let meta_path = join(&rag_path, "meta.json").unwrap();
    let meta_json = read_string(&meta_path).unwrap_or(String::from("{}"));
    Box::new(with_header(
        meta_json,
        "Content-Type",
        "application/json",
    ))
}

pub fn get_version(user: String, repo: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);
    let index_path = join(&rag_path, "index.json").unwrap();

    if !exists(&index_path) {
        return Box::new(with_status(
            String::new(),
            StatusCode::from_u16(404).unwrap(),
        ));
    }

    let index_json = read_string(&index_path).unwrap_or(String::from("{}"));
    let index = serde_json::from_str::<Value>(&index_json).unwrap_or(Value::Object(Map::new()));

    match index {
        Value::Object(obj) => match obj.get("ragit_version") {
            Some(v) => match v.as_str() {
                Some(v) => Box::new(with_header(
                    v.to_string(),
                    "Content-Type",
                    "text/plain",
                )),
                None => Box::new(with_status(String::new(), StatusCode::from_u16(500).unwrap())),
            },
            None => Box::new(with_status(String::new(), StatusCode::from_u16(500).unwrap())),
        },
        _ => Box::new(with_status(String::new(), StatusCode::from_u16(500).unwrap())),
    }
}

pub fn get_server_version() -> Box<dyn Reply> {
    Box::new(with_header(
        ragit::VERSION,
        "Content-Type",
        "text/plain",
    ))
}
