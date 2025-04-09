use super::{HandleError, RawResponse, get_pool, handler};
use crate::CONFIG;
use crate::models::{ai_model, chat, repo, user};
use ragit::{Index, LoadMode, QueryResponse, QueryTurn};
use ragit_fs::join3;
use std::collections::HashMap;
use warp::reply::{Reply, json, with_header};

pub async fn get_chat(user: String, repo: String, chat_id: String) -> Box<dyn Reply> {
    handler(get_chat_(user, repo, chat_id).await)
}

async fn get_chat_(user: String, repo: String, chat_id: String) -> RawResponse {
    let pool = get_pool().await;
    let repo_id = repo::get_id_by_name(&user, &repo, pool).await.handle_error(404)?;
    let chat_id = chat_id.parse::<i32>().handle_error(400)?;
    let chat = chat::get_chat_with_history_by_id(chat_id, pool).await.handle_error(404)?;

    if chat.repo_id != repo_id {
        return Err((400, format!("chat {chat_id} does not belong to {repo_id}")));
    }

    Ok(Box::new(json(&chat)))
}

pub async fn get_chat_list(user: String, repo: String, query: HashMap<String, String>) -> Box<dyn Reply> {
    handler(get_chat_list_(user, repo, query).await)
}

async fn get_chat_list_(user: String, repo: String, query: HashMap<String, String>) -> RawResponse {
    let pool = get_pool().await;
    let limit = query.get("limit").map(|s| s.as_ref()).unwrap_or("50").parse::<i64>().unwrap_or(50);
    let offset = query.get("offset").map(|s| s.as_ref()).unwrap_or("0").parse::<i64>().unwrap_or(0);
    let repo_id = repo::get_id_by_name(&user, &repo, pool).await.handle_error(404)?;
    let chats = chat::get_list_by_repo_id(repo_id, limit, offset, pool).await.handle_error(500)?;

    Ok(Box::new(json(&chats)))
}

pub async fn post_chat(user: String, repo: String, chat_id: String, form: HashMap<String, Vec<u8>>) -> Box<dyn Reply> {
    handler(post_chat_(user, repo, chat_id, form).await)
}

async fn post_chat_(user: String, repo: String, chat_id: String, form: HashMap<String, Vec<u8>>) -> RawResponse {
    let pool = get_pool().await;
    let config = CONFIG.get().unwrap();
    let query = match form.get("query") {
        Some(query) => String::from_utf8_lossy(query).to_string(),
        None => {
            return Err((400, String::from("`query` field is missing")));
        },
    };
    let index_at = join3(
        &config.repo_data_dir,
        &user,
        &repo,
    ).handle_error(400)?;

    let user_id = user::get_id_by_name(&user, pool).await.handle_error(404)?;
    let repo_id = repo::get_id_by_name(&user, &repo, pool).await.handle_error(404)?;
    let chat_id = chat_id.parse::<i32>().handle_error(400)?;
    let chat = chat::get_chat_by_id(chat_id, pool).await.handle_error(404)?;

    if chat.repo_id != repo_id {
        return Err((400, format!("chat {chat_id} does not belong to {repo_id}")));
    }

    let mut model_name = ai_model::get_default_model_name(user_id, pool).await.handle_error(500)?;

    if let Some(model) = form.get("model") {
        model_name = String::from_utf8(model.to_vec()).handle_error(400)?;
    }

    // TODO: I want it to return more detailed error message
    let model_schema = ai_model::get_model_schema(user_id, &model_name, pool).await.handle_error(400)?;

    // There's a quirk. Ragit reads model info from `.ragit/models.json`, but ragit-server wants to
    // store everything on DB. And it does so. So, it first reads model info from the DB and
    // writes `.ragit/models.json` on the fly.
    ai_model::update_model_schema(&user, &repo, &model_schema).handle_error(500)?;
    let mut index = Index::load(index_at, LoadMode::OnlyJson).handle_error(500)?;
    index.api_config.model = model_name.clone();
    let history = chat::get_history_by_id(chat_id, pool).await.handle_error(500)?;
    let mut real_history = Vec::with_capacity(history.len());

    for h in history.iter() {
        real_history.push(QueryTurn {
            query: h.query.to_string(),
            response: QueryResponse {
                response: h.response.to_string(),
                multi_turn_schema: h.multi_turn_schema.clone(),

                // NOTE: `index.query` doesn't read this field
                retrieved_chunks: vec![],
            },
        });
    }

    let response = index.query(&query, real_history).await.handle_error(500)?;

    chat::add_chat_history(
        chat_id,
        &query,
        &history,
        &response,
        0,  // TODO: user_id
        &model_name,
        pool,
    ).await.handle_error(500)?;

    Ok(Box::new(json(&response)))
}

pub async fn create_chat(user: String, repo: String) -> Box<dyn Reply> {
    handler(create_chat_(user, repo).await)
}

async fn create_chat_(user: String, repo: String) -> RawResponse {
    let pool = get_pool().await;
    let repo_id = repo::get_id_by_name(&user, &repo, pool).await.handle_error(404)?;
    let chat_id = chat::create_and_return_id(repo_id, pool).await.handle_error(500)?;

    Ok(Box::new(with_header(
        chat_id.to_string(),
        "Content-Type",
        "text/plain; charset=utf-8",
    )))
}
