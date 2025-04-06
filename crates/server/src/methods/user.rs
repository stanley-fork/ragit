use super::{HandleError, RawResponse, get_pool, handler};
use crate::models::user::{self, UserCreate};
use std::collections::HashMap;
use warp::reply::{Reply, json};

pub async fn get_user_list(query: HashMap<String, String>) -> Box<dyn Reply> {
    handler(get_user_list_(query).await)
}

async fn get_user_list_(query: HashMap<String, String>) -> RawResponse {
    let pool = &get_pool().await;
    let limit = query.get("limit").map(|s| s.as_ref()).unwrap_or("50").parse::<i64>().unwrap_or(50);
    let offset = query.get("offset").map(|s| s.as_ref()).unwrap_or("0").parse::<i64>().unwrap_or(0);
    let users = user::get_list(limit, offset, pool).await.handle_error(500)?;

    Ok(Box::new(json(&users)))
}

pub async fn create_user(body: HashMap<String, String>) -> Box<dyn Reply> {
    handler(create_user_(body).await)
}

async fn create_user_(body: HashMap<String, String>) -> RawResponse {
    let pool = &get_pool().await;
    let user = serde_json::to_value(&body).handle_error(400)?;
    let user = serde_json::from_value::<UserCreate>(user).handle_error(400)?;
    let user_id = user::create_and_return_id(&user, pool).await.handle_error(500)?;
    Ok(Box::new(json(&user_id)))
}
