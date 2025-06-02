use super::{HandleError, RawResponse, get_or, get_pool, handler};
use crate::models::auth;
use crate::models::ai_model::{self, AiModelCreation};
use serde_json::Value;
use std::collections::HashMap;
use warp::http::StatusCode;
use warp::reply::{Reply, json, with_status};

pub async fn get_ai_model_list(query: HashMap<String, String>) -> Box<dyn Reply> {
    handler(get_ai_model_list_(query).await)
}

async fn get_ai_model_list_(query: HashMap<String, String>) -> RawResponse {
    let pool = get_pool().await;

    let name = query.get("name").map(|n| n.to_string());
    let tags = match query.get("tags") {
        Some(tags) => tags.split(",").map(
            |tag| tag.trim().to_string()
        ).filter(
            |tag| !tag.is_empty()
        ).collect(),
        None => vec![],
    };

    let limit = get_or(&query, "limit", 50);
    let offset = get_or(&query, "offset", 0);
    let ai_models = ai_model::get_list(name, tags, limit, offset, pool).await.handle_error(500)?;
    Ok(Box::new(json(&ai_models)))
}

pub async fn put_ai_model_list(model: Value, api_key: Option<String>) -> Box<dyn Reply> {
    handler(put_ai_model_list_(model, api_key).await)
}

async fn put_ai_model_list_(model: Value, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;

    // only admin can upsert an ai model
    auth::is_admin(api_key, pool).await.handle_error(500)?.handle_error(403)?;

    let model = serde_json::from_value::<AiModelCreation>(model).handle_error(400)?;
    ai_model::upsert_and_return_id(&model, pool).await.handle_error(500)?;
    Ok(Box::new(with_status(
        String::new(),
        StatusCode::from_u16(200).unwrap(),
    )))
}
