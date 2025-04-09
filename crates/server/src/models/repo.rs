use chrono::{DateTime, Utc};
use chrono::serde::{ts_milliseconds, ts_milliseconds_option};
use crate::error::Error;
use crate::utils::normalize;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;

#[derive(Clone, Debug, Serialize)]
pub struct RepoDetail {
    pub id: i32,
    pub name: String,
    pub normalized_name: String,
    pub owner_name: String,
    pub owner_normalized_name: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub stars: i32,
    pub readme: Option<String>,
    pub repo_size: i64,
    #[serde(with = "ts_milliseconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ts_milliseconds_option")]
    pub pushed_at: Option<DateTime<Utc>>,
    #[serde(with = "ts_milliseconds")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RepoSimple {
    pub id: i32,
    pub name: String,
    pub owner_name: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub stars: i32,
    pub repo_size: i64,
    #[serde(with = "ts_milliseconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ts_milliseconds_option")]
    pub pushed_at: Option<DateTime<Utc>>,
    #[serde(with = "ts_milliseconds")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RepoCreate {
    pub name: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub readme: Option<String>,
    pub public_read: bool,
    pub public_write: bool,
    pub public_clone: bool,
    pub public_push: bool,
}

pub async fn get_id_by_name(user_name: &str, repo_name: &str, pool: &PgPool) -> Result<i32, Error> {
    // `normalize` is idempotent
    let user_name = normalize(user_name);
    let repo_name = normalize(repo_name);

    let row = crate::query!(
        "SELECT repository.id FROM repository JOIN user_ ON user_.normalized_name = $1 WHERE owner_id = user_.id AND repository.normalized_name = $2",
        user_name,
        repo_name,
    ).fetch_one(pool).await?;

    Ok(row.id)
}

pub async fn get_list(user_id: i32, limit: i64, offset: i64, pool: &PgPool) -> Result<Vec<RepoSimple>, Error> {
    let rows = crate::query!(
        "
        SELECT
            repository.id,
            repository.name AS repo_name,
            user_.name AS user_name,
            description,
            website,
            stars,
            repo_size,
            repository.created_at,
            repository.pushed_at,
            repository.updated_at
        FROM repository
        JOIN user_ ON user_.id = repository.owner_id
        WHERE owner_id = $1 ORDER BY updated_at DESC LIMIT $2 OFFSET $3",
        user_id,
        limit,
        offset,
    ).fetch_all(pool).await?;
    let mut result = Vec::with_capacity(rows.len());

    for row in rows.iter() {
        result.push(RepoSimple {
            id: row.id,
            name: row.repo_name.clone(),
            owner_name: row.user_name.clone(),
            description: row.description.clone(),
            website: row.website.clone(),
            stars: row.stars,
            repo_size: row.repo_size,
            created_at: row.created_at,
            pushed_at: row.pushed_at,
            updated_at: row.updated_at,
        });
    }

    Ok(result)
}

pub async fn get_detail(repo_id: i32, pool: &PgPool) -> Result<RepoDetail, Error> {
    let row = crate::query!(
        "SELECT
            repository.id,
            repository.name as name,
            repository.normalized_name as normalized_name,
            user_.name as owner_name,
            user_.normalized_name as owner_normalized_name,
            description,
            website,
            stars,
            repository.readme,
            repo_size,
            repository.created_at,
            pushed_at,
            updated_at
        FROM repository JOIN user_ ON repository.owner_id = user_.id
        WHERE repository.id = $1",
        repo_id,
    ).fetch_one(pool).await?;

    Ok(RepoDetail {
        id: row.id,
        name: row.name,
        normalized_name: row.normalized_name,
        owner_name: row.owner_name,
        owner_normalized_name: row.owner_normalized_name,
        description: row.description,
        website: row.website,
        stars: row.stars,
        readme: row.readme,
        repo_size: row.repo_size,
        created_at: row.created_at,
        pushed_at: row.pushed_at,
        updated_at: row.updated_at,
    })
}

pub async fn create_and_return_id(user_id: i32, repo: &RepoCreate, pool: &PgPool) -> Result<i32, Error> {
    let repo_id = crate::query!(
        "INSERT
        INTO repository (
            owner_id,
            name,
            normalized_name,
            description,
            website,
            stars,
            readme,
            public_read,
            public_write,
            public_clone,
            public_push,
            chunk_count,
            repo_size,
            push_session_id,
            created_at,
            pushed_at,
            updated_at
        )
        VALUES (
            $1,    -- owner_id
            $2,    -- name
            $3,    -- normalized_name
            $4,    -- description
            $5,    -- website
            0,     -- stars
            $6,    -- readme
            $7,    -- public_read
            $8,    -- public_write
            $9,    -- public_clone
            $10,   -- public_push
            0,     -- chunk_count
            0,     -- repo_size
            NULL,  -- push_session_id
            NOW(), -- created_at
            NULL,  -- pushed_at
            NOW()  -- updated_at
        )
        RETURNING id",
        user_id,
        repo.name.clone(),
        normalize(&repo.name),
        repo.description.clone(),
        repo.website.clone(),
        repo.readme.clone(),
        repo.public_read,
        repo.public_write,
        repo.public_clone,
        repo.public_push,
    ).fetch_one(pool).await?.id;

    Ok(repo_id)
}

pub async fn get_session_id(user: &str, repo: &str, pool: &PgPool) -> Result<Option<String>, Error> {
    let repo_id = get_id_by_name(user, repo, pool).await?;
    let session_id = crate::query!(
        "SELECT push_session_id FROM repository WHERE id = $1",
        repo_id,
    ).fetch_one(pool).await?.push_session_id;

    Ok(session_id)
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Traffic {
    pub push: u64,
    pub clone: u64,
}

pub async fn get_traffic_by_key(repo_id: i32, key: &str, pool: &PgPool) -> Result<Traffic, Error> {
    let maybe_row = crate::query!(
        "SELECT push, clone FROM repository_stat WHERE repo_id = $1 AND date_str = $2",
        repo_id,
        key,
    ).fetch_all(pool).await?;

    match maybe_row.get(0) {
        Some(row) => Ok(Traffic { push: row.push as u64, clone: row.clone as u64 }),
        None => Ok(Traffic { push: 0, clone: 0 }),
    }
}

pub async fn get_traffic_all(repo_id: i32, pool: &PgPool) -> Result<Traffic, Error> {
    let row = crate::query!(
        "SELECT SUM(push) AS push, SUM(clone) AS clone FROM repository_stat WHERE repo_id = $1",
        repo_id,
    ).fetch_one(pool).await?;

    Ok(Traffic { push: row.push.unwrap_or(0) as u64, clone: row.clone.unwrap_or(0) as u64 })
}
