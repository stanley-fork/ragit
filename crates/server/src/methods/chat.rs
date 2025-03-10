use super::{HandleError, RawResponse, handler};
use chrono::Local;
use crate::models::Chat;
use ragit::{Index, LoadMode, QueryTurn};
use ragit_fs::{
    WriteMode,
    create_dir_all,
    exists,
    extension,
    join,
    join3,
    join5,
    read_dir,
    set_extension,
    write_string,
};
use std::collections::HashMap;
use warp::reply::{Reply, json, with_header};

pub fn get_chat(user: String, repo: String, chat_id: String) -> Box<dyn Reply> {
    handler(get_chat_(user, repo, chat_id))
}

fn get_chat_(user: String, repo: String, chat_id: String) -> RawResponse {
    let chat_at = join5(
        "data",
        &user,
        &repo,
        "chats",
        &set_extension(&chat_id, "json").handle_error(400)?,
    ).handle_error(400)?;
    Ok(Box::new(json(&Chat::load_from_file(&chat_at).handle_error(404)?)))
}

pub fn get_chat_list(user: String, repo: String, query: HashMap<String, String>) -> Box<dyn Reply> {
    handler(get_chat_list_(user, repo, query))
}

fn get_chat_list_(user: String, repo: String, query: HashMap<String, String>) -> RawResponse {
    let no_history = query.get("history").map(|s| s.as_ref()).unwrap_or("") == "0";
    let repo_at = join3("data", &user, &repo).handle_error(400)?;

    if !exists(&repo_at) {
        return Err((404, format!("`/{user}/{repo}` not found")));
    }

    let chats_at = join(&repo_at, "chats").handle_error(400)?;

    if !exists(&chats_at) {
        return Ok(Box::new(json::<Vec<Chat>>(&vec![])));
    }

    let mut result = vec![];

    for file in read_dir(&chats_at, true).handle_error(404)? {
        match extension(&file) {
            Ok(Some(e)) if e == "json" => {
                let mut chat = Chat::load_from_file(&file).handle_error(500)?;

                if no_history {
                    chat.history = vec![];
                }

                result.push(chat);
            },
            _ => {},
        }
    }

    Ok(Box::new(json(&result)))
}

pub async fn post_chat(user: String, repo: String, chat_id: String, form: HashMap<String, Vec<u8>>) -> Box<dyn Reply> {
    handler(post_chat_(user, repo, chat_id, form).await)
}

async fn post_chat_(user: String, repo: String, chat_id: String, form: HashMap<String, Vec<u8>>) -> RawResponse {
    let query = match form.get("query") {
        Some(query) => String::from_utf8_lossy(query).to_string(),
        None => {
            return Err((400, String::from("`query` field is missing")));
        },
    };
    let model = match form.get("model") {
        Some(model) => Some(String::from_utf8_lossy(model).to_string()),
        None => None,
    };
    let index_at = join3(
        "data",
        &user,
        &repo,
    ).handle_error(400)?;
    let chat_at = join3(
        &index_at,
        "chats",
        &set_extension(&chat_id, "json").handle_error(400)?,
    ).handle_error(400)?;

    let mut chat = Chat::load_from_file(&chat_at).handle_error(404)?;
    let mut index = Index::load(index_at, LoadMode::OnlyJson).handle_error(500)?;

    if let Some(model) = model {
        index.api_config.model = model;
    }

    let response = index.query(&query, chat.history.clone()).await.handle_error(500)?;
    chat.history.push(QueryTurn { query, response: response.clone() });
    chat.updated_at = Local::now().timestamp();
    chat.save_to_file(&chat_at).handle_error(500)?;

    Ok(Box::new(json(&response)))
}

pub fn create_chat(user: String, repo: String) -> Box<dyn Reply> {
    handler(create_chat_(user, repo))
}

fn create_chat_(user: String, repo: String) -> RawResponse {
    let index_at = join3(
        "data",
        &user,
        &repo,
    ).handle_error(400)?;

    if !exists(&index_at) {
        return Err((404, format!("`{index_at}` not found")));
    }

    let chat_at = join(&index_at, "chats").handle_error(500)?;

    if !exists(&chat_at) {
        create_dir_all(&chat_at).handle_error(500)?;
    }

    let id = format!("{:016x}", rand::random::<u64>());
    let chat = Chat::new(id.clone());
    write_string(
        &join(&chat_at, &set_extension(&id, "json").handle_error(500)?).handle_error(500)?,
        &serde_json::to_string(&chat).handle_error(500)?,
        WriteMode::AlwaysCreate,
    ).handle_error(500)?;

    Ok(Box::new(with_header(
        id,
        "Content-Type",
        "text/plain; charset=utf-8",
    )))
}
