use super::{HandleError, RawResponse, get_or, get_pool, handler};
use crate::CONFIG;
use crate::models::auth;
use crate::models::ai_model::{self, UserAiModelUpdate};
use crate::models::user::{self, UserCreation};
use serde_json::Value;
use std::collections::HashMap;
use warp::http::StatusCode;
use warp::reply::{Reply, json, with_status};

pub async fn get_user_list(query: HashMap<String, String>, api_key: Option<String>) -> Box<dyn Reply> {
    handler(get_user_list_(query, api_key).await)
}

async fn get_user_list_(query: HashMap<String, String>, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let limit = get_or(&query, "limit", 50);
    let offset = get_or(&query, "offset", 0);
    let include_privates = auth::is_admin(api_key, pool).await.handle_error(500)?;
    let users = user::get_list(include_privates, limit, offset, pool).await.handle_error(500)?;

    // TODO: if a private user X requests this api with his api key, it should include X but doesn't

    Ok(Box::new(json(&users)))
}

pub async fn get_user(user: String, api_key: Option<String>) -> Box<dyn Reply> {
    handler(get_user_(user, api_key).await)
}

async fn get_user_(user: String, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let user = user::get_detail(&user, pool).await.handle_error(404)?;

    if !user.public {
        user::check_auth(&user.id, api_key, pool).await.handle_error(500)?.handle_error(404)?;
    }

    Ok(Box::new(json(&user)))
}

pub async fn create_user(body: Value, api_key: Option<String>) -> Box<dyn Reply> {
    handler(create_user_(body, api_key).await)
}

async fn create_user_(body: Value, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let config = CONFIG.get().handle_error(500)?;
    let no_user_at_all = user::no_user_at_all(pool).await.handle_error(500)?;

    if !no_user_at_all && config.only_admin_can_create_user {
        auth::is_admin(api_key, pool).await.handle_error(500)?.handle_error(403)?;
    }

    let user = serde_json::from_value::<UserCreation>(body).handle_error(400)?;
    user::create(&user, pool).await.handle_error(500)?;
    Ok(Box::new(with_status(String::new(), StatusCode::from_u16(200).unwrap())))
}

pub async fn get_user_ai_model_list(user: String, api_key: Option<String>) -> Box<dyn Reply> {
    handler(get_user_ai_model_list_(user, api_key).await)
}

async fn get_user_ai_model_list_(user: String, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;

    // TODO: do I have to allow everyone to see the model list of a public user?
    user::check_auth(&user, api_key, pool).await.handle_error(500)?.handle_error(404)?;

    let model_list = ai_model::get_list_by_user_id(&user, pool).await.handle_error(500)?;
    Ok(Box::new(json(&model_list)))
}

pub async fn put_user_ai_model_list(user: String, form: Value, api_key: Option<String>) -> Box<dyn Reply> {
    handler(put_user_ai_model_list_(user, form, api_key).await)
}

async fn put_user_ai_model_list_(user: String, form: Value, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    user::check_auth(&user, api_key, pool).await.handle_error(500)?.handle_error(404)?;
    let update = serde_json::from_value::<UserAiModelUpdate>(form).handle_error(400)?;
    ai_model::register(&user, &update, pool).await.handle_error(500)?;

    Ok(Box::new(with_status(
        String::new(),
        StatusCode::from_u16(200).unwrap(),
    )))
}
