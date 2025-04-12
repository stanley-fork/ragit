use super::{HandleError, RawResponse, get_pool, handler};
use chrono::{Datelike, Utc};
use crate::models::{repo, user};
use crate::models::repo::{RepoCreate, RepoOperation};
use serde_json::Value;
use std::collections::HashMap;
use warp::reply::{Reply, json};

pub async fn get_repo_list(user: String, query: HashMap<String, String>, api_key: Option<String>) -> Box<dyn Reply> {
    handler(get_repo_list_(user, query, api_key).await)
}

async fn get_repo_list_(user: String, query: HashMap<String, String>, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let limit = query.get("limit").map(|s| s.as_ref()).unwrap_or("50").parse::<i64>().unwrap_or(50);
    let offset = query.get("offset").map(|s| s.as_ref()).unwrap_or("0").parse::<i64>().unwrap_or(0);
    let user_id = user::get_id_by_name(&user, pool).await.handle_error(404)?;
    let has_permission = user::check_auth(user_id, api_key, pool).await.handle_error(500)?;
    let repo_list = repo::get_list(user_id, has_permission, limit, offset, pool).await.handle_error(500)?;

    Ok(Box::new(json(&repo_list)))
}

pub async fn get_repo(user: String, repo: String, api_key: Option<String>) -> Box<dyn Reply> {
    handler(get_repo_(user, repo, api_key).await)
}

async fn get_repo_(user: String, repo: String, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let repo_id = repo::get_id_by_name(&user, &repo, pool).await.handle_error(404)?;

    // 500: DB error, 404: no permission but returns 404 instead of 403 for security reasons
    repo::check_auth(repo_id, RepoOperation::Read, api_key, pool).await.handle_error(500)?.handle_error(404)?;
    let repo = repo::get_detail(repo_id, pool).await.handle_error(500)?;

    Ok(Box::new(json(&repo)))
}

// I set `body: Value` not `body: RepoCreate` because it gives a better error message for invalid schemas.
pub async fn create_repo(user: String, body: Value, api_key: Option<String>) -> Box<dyn Reply> {
    handler(create_repo_(user, body, api_key).await)
}

async fn create_repo_(user: String, body: Value, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let repo = serde_json::from_value::<RepoCreate>(body).handle_error(400)?;
    let user_id = user::get_id_by_name(&user, pool).await.handle_error(404)?;
    user::check_auth(user_id, api_key, pool).await.handle_error(500)?.handle_error(403)?;
    let repo_id = repo::create_and_return_id(user_id, &repo, pool).await.handle_error(500)?;
    Ok(Box::new(json(&repo_id)))
}

pub async fn get_traffic(user: String, repo: String, api_key: Option<String>) -> Box<dyn Reply> {
    handler(get_traffic_(user, repo, api_key).await)
}

async fn get_traffic_(user: String, repo: String, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let repo_id = repo::get_id_by_name(&user, &repo, pool).await.handle_error(404)?;

    // 500: DB error, 404: no permission but returns 404 instead of 403 for security reasons
    repo::check_auth(repo_id, RepoOperation::Read, api_key, pool).await.handle_error(500)?.handle_error(404)?;

    let mut now = Utc::now();
    let mut result = HashMap::new();

    for _ in 0..14 {
        let (year, month, day) = (now.year(), now.month(), now.day());
        let key = format!("{year:04}-{month:02}-{day:02}");
        result.insert(
            key.clone(),
            repo::get_traffic_by_key(repo_id, &key, pool).await.handle_error(500)?,
        );
        now = now.checked_sub_days(chrono::Days::new(1)).handle_error(500)?;
    }

    result.insert(
        String::from("all"),
        repo::get_traffic_all(repo_id, pool).await.handle_error(500)?,
    );
    Ok(Box::new(json(&result)))
}
