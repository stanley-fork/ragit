use super::{HandleError, RawResponse, get_pool, handler};
use crate::models::{repo, user};
use crate::models::repo::RepoCreate;
use serde_json::Value;
use std::collections::HashMap;
use warp::reply::{Reply, json};

pub async fn get_repo_list(user: String, query: HashMap<String, String>) -> Box<dyn Reply> {
    handler(get_repo_list_(user, query).await)
}

async fn get_repo_list_(user: String, query: HashMap<String, String>) -> RawResponse {
    let pool = get_pool().await;
    let limit = query.get("limit").map(|s| s.as_ref()).unwrap_or("50").parse::<i64>().unwrap_or(50);
    let offset = query.get("offset").map(|s| s.as_ref()).unwrap_or("0").parse::<i64>().unwrap_or(0);
    let user_id = user::get_id_by_name(&user, pool).await.handle_error(404)?;
    let repos = repo::get_list(user_id, limit, offset, pool).await.handle_error(500)?;

    Ok(Box::new(json(&repos)))
}

pub async fn create_repo(user: String, body: Value) -> Box<dyn Reply> {
    handler(create_repo_(user, body).await)
}

async fn create_repo_(user: String, body: Value) -> RawResponse {
    let pool = get_pool().await;
    let repo = serde_json::from_value::<RepoCreate>(body).handle_error(400)?;
    let user_id = user::get_id_by_name(&user, pool).await.handle_error(404)?;
    let repo_id = repo::create_and_return_id(user_id, &repo, pool).await.handle_error(500)?;
    Ok(Box::new(json(&repo_id)))
}
