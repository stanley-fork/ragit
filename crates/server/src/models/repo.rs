use super::auth;
use chrono::{DateTime, Utc};
use chrono::serde::{ts_milliseconds, ts_milliseconds_option};
use crate::error::Error;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;

pub enum RepoOperation {
    Read,
    Write,
    Clone,
    Push,
    Chat,

    // whatever sensitive operation that requires an api key
    Sensitive,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RepoDetail {
    pub id: i32,
    pub name: String,
    pub owner: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub stars: i32,
    pub readme: Option<String>,
    pub repo_size: i64,  // sum of the size of its archives, in bytes
    #[serde(with = "ts_milliseconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ts_milliseconds_option")]
    pub pushed_at: Option<DateTime<Utc>>,
    #[serde(with = "ts_milliseconds_option")]
    pub search_index_built_at: Option<DateTime<Utc>>,
    #[serde(with = "ts_milliseconds")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RepoSimple {
    pub id: i32,
    pub name: String,
    pub owner: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub stars: i32,
    pub repo_size: i64,  // sum of the size of its archives, in bytes
    #[serde(with = "ts_milliseconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ts_milliseconds_option")]
    pub pushed_at: Option<DateTime<Utc>>,
    #[serde(with = "ts_milliseconds_option")]
    pub search_index_built_at: Option<DateTime<Utc>>,
    #[serde(with = "ts_milliseconds")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RepoCreate {
    pub name: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub readme: Option<String>,
    pub public_read: bool,
    pub public_write: bool,
    pub public_clone: bool,
    pub public_push: bool,
    pub public_chat: bool,
}

// TODO: allow change name?
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RepoUpdate {
    pub description: Option<String>,
    pub website: Option<String>,
    pub readme: Option<String>,
    pub public_read: bool,
    pub public_write: bool,
    pub public_clone: bool,
    pub public_push: bool,
    pub public_chat: bool,
}

pub async fn get_id(user: &str, repo: &str, pool: &PgPool) -> Result<i32, Error> {
    let row = crate::query!(
        "SELECT repository.id FROM repository JOIN user_ ON user_.id = $1 WHERE owner = user_.id AND repository.name = $2",
        user,
        repo,
    ).fetch_one(pool).await?;

    Ok(row.id)
}

pub async fn get_list(
    user: &str,
    has_permission: bool,
    limit: i64,
    offset: i64,
    pool: &PgPool,
) -> Result<Vec<RepoSimple>, Error> {
    let rows = crate::query!(
        "SELECT
            repository.id,
            repository.name AS repo_name,
            repository.owner,
            description,
            website,
            stars,
            (SELECT SUM(blob_size) FROM archive WHERE session_id = repository.push_session_id) AS repo_size,
            public_read,
            repository.created_at,
            repository.pushed_at,
            repository.search_index_built_at,
            repository.updated_at
        FROM repository
        JOIN user_ ON user_.id = repository.owner
        WHERE owner = $1 ORDER BY updated_at DESC LIMIT $2 OFFSET $3",
        user,
        limit,
        offset,
    ).fetch_all(pool).await?;
    let mut result = Vec::with_capacity(rows.len());

    for row in rows.iter() {
        if !has_permission && !row.public_read {
            continue;
        }

        result.push(RepoSimple {
            id: row.id,
            name: row.repo_name.clone(),
            owner: row.owner.clone(),
            description: row.description.clone(),
            website: row.website.clone(),
            stars: row.stars,
            repo_size: row.repo_size.unwrap_or(0),
            created_at: row.created_at,
            pushed_at: row.pushed_at,
            search_index_built_at: row.search_index_built_at,
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
            repository.owner,
            description,
            website,
            stars,
            repository.readme,
            (SELECT SUM(blob_size) FROM archive WHERE session_id = repository.push_session_id) AS repo_size,
            repository.created_at,
            pushed_at,
            search_index_built_at,
            updated_at
        FROM repository JOIN user_ ON repository.owner = user_.id
        WHERE repository.id = $1",
        repo_id,
    ).fetch_one(pool).await?;

    Ok(RepoDetail {
        id: row.id,
        name: row.name,
        owner: row.owner,
        description: row.description,
        website: row.website,
        stars: row.stars,
        readme: row.readme,
        repo_size: row.repo_size.unwrap_or(0),
        created_at: row.created_at,
        pushed_at: row.pushed_at,
        search_index_built_at: row.search_index_built_at,
        updated_at: row.updated_at,
    })
}

pub async fn create_and_return_id(user: &str, repo: &RepoCreate, pool: &PgPool) -> Result<i32, Error> {
    let repo_id = crate::query!(
        "INSERT
        INTO repository (
            owner,
            name,
            description,
            website,
            stars,
            readme,
            public_read,
            public_write,
            public_clone,
            public_push,
            public_chat,
            chunk_count,
            push_session_id,
            created_at,
            pushed_at,
            search_index_built_at,
            updated_at
        )
        VALUES (
            $1,    -- owner
            $2,    -- name
            $3,    -- description
            $4,    -- website
            0,     -- stars
            $5,    -- readme
            $6,    -- public_read
            $7,    -- public_write
            $8,    -- public_clone
            $9,   -- public_push
            $10,   -- public_chat
            0,     -- chunk_count
            NULL,  -- push_session_id
            NOW(), -- created_at
            NULL,  -- pushed_at
            NULL,  -- search_index_built_at
            NOW()  -- updated_at
        )
        RETURNING id",
        user,
        repo.name.clone(),
        repo.description.clone(),
        repo.website.clone(),
        repo.readme.clone(),
        repo.public_read,
        repo.public_write,
        repo.public_clone,
        repo.public_push,
        repo.public_chat,
    ).fetch_one(pool).await?.id;

    Ok(repo_id)
}

pub async fn update_repo(repo_id: i32, repo: RepoUpdate, pool: &PgPool) -> Result<(), Error> {
    crate::query!(
        "UPDATE repository
        SET
            description = $1,
            website = $2,
            readme = $3,
            public_read = $4,
            public_write = $5,
            public_clone = $6,
            public_push = $7,
            public_chat = $8
        WHERE id = $9",
        repo.description.as_ref().map(|s| s.as_str()),
        repo.website.as_ref().map(|s| s.as_str()),
        repo.readme.as_ref().map(|s| s.as_str()),
        repo.public_read,
        repo.public_write,
        repo.public_clone,
        repo.public_push,
        repo.public_chat,
        repo_id,
    ).execute(pool).await?;
    Ok(())
}

pub async fn get_session_id(repo_id: i32, pool: &PgPool) -> Result<Option<String>, Error> {
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

pub async fn check_auth(
    repo_id: i32,
    operation: RepoOperation,
    api_key: Option<String>,
    pool: &PgPool,
) -> Result<bool, Error> {
    let row = crate::query!(
        "SELECT owner, public_read, public_write, public_clone, public_push, public_chat FROM repository WHERE id = $1",
        repo_id,
    ).fetch_one(pool).await?;
    let (
        public_read,
        public_write,
        public_clone,
        public_push,
        public_chat,
    ) = (
        row.public_read,
        row.public_write,
        row.public_clone,
        row.public_push,
        row.public_chat,
    );

    match (operation, public_read, public_write, public_clone, public_push, public_chat) {
        (RepoOperation::Read, true, _, _, _, _)
        | (RepoOperation::Write, _, true, _, _, _)
        | (RepoOperation::Clone, _, _, true, _, _)
        | (RepoOperation::Push, _, _, _, true, _)
        | (RepoOperation::Chat, _, _, _, _, true) => {
            return Ok(true);
        },
        _ if api_key.is_none() => {
            return Ok(false);
        },
        _ => {},
    }

    let permission = auth::get_user_id_from_api_key(api_key, pool).await?;

    match permission {
        Some(auth::Permission { user, is_admin }) if user == row.owner || is_admin => Ok(true),
        _ => Ok(false),
    }
}

pub async fn update_search_index_build_time(repo_id: i32, pool: &PgPool) -> Result<(), Error> {
    crate::query!(
        "UPDATE repository SET search_index_built_at = NOW() WHERE id = $1",
        repo_id,
    ).execute(pool).await?;
    Ok(())
}
