use base64::Engine;
use base64::prelude::BASE64_URL_SAFE as base64_engine;
use chrono::{DateTime, Utc};
use crate::error::Error;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use sqlx::postgres::PgPool;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Permission {
    pub user: String,
    pub is_admin: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApiKey {
    pub api_key_preview: String,
    pub name: String,
    pub expire: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApiKeyCreation {
    pub name: String,
    pub expire_after: u64,  // days

    // user password in plain text
    pub password: String,
}

pub async fn get_user_id_from_api_key(api_key: Option<String>, pool: &PgPool) -> Result<Option<Permission>, Error> {
    let Some(api_key) = api_key else { return Ok(None); };
    let rows = crate::query!(
        "SELECT user_, is_admin FROM api_key JOIN user_ ON api_key.user_ = user_.id WHERE api_key = $1 AND expire > NOW()",
        api_key,
    ).fetch_all(pool).await?;

    match rows.get(0) {
        Some(row) => Ok(Some(Permission { user: row.user_.to_string(), is_admin: row.is_admin })),
        None => Ok(None),
    }
}

pub async fn create_api_key(user: &str, name: &str, expire: DateTime<Utc>, pool: &PgPool) -> Result<String, Error> {
    let key = format!("rgs_{}", encode_base64(&(0..240).map(|_| rand::random::<u8>()).collect::<Vec<_>>()));
    crate::query!(
        "INSERT INTO api_key (api_key, name, user_, expire) VALUES ($1, $2, $3, $4)",
        &key,
        name,
        user,
        expire,
    ).execute(pool).await?;
    Ok(key)
}

pub async fn get_api_key_list(user: &str, pool: &PgPool) -> Result<Vec<ApiKey>, Error> {
    let rows = crate::query!(
        "SELECT api_key, name, expire FROM api_key WHERE user_ = $1",
        user,
    ).fetch_all(pool).await?;
    let mut result = Vec::with_capacity(rows.len());

    for row in rows.iter() {
        result.push(ApiKey {
            api_key_preview: row.api_key.get(0..9).ok_or(Error::InvalidUtf8)?.to_string(),
            name: row.name.clone(),
            expire: row.expire,
        });
    }

    Ok(result)
}

pub async fn check_password(user: &str, password: &str, pool: &PgPool) -> Result<bool, Error> {
    let row = crate::query!(
        "SELECT salt, password FROM user_ WHERE id = $1",
        user,
    ).fetch_one(pool).await?;
    let (salt, password_hash) = (&row.salt, &row.password);

    Ok(&hash_password(salt, password) == password_hash)
}

pub async fn is_admin(api_key: Option<String>, pool: &PgPool) -> Result<bool, Error> {
    let Some(api_key) = api_key else { return Ok(false); };
    let rows = crate::query!(
        "SELECT is_admin FROM user_ JOIN api_key ON api_key.user_ = user_.id WHERE api_key.api_key = $1",
        api_key,
    ).fetch_all(pool).await?;

    match rows.get(0) {
        Some(row) => Ok(row.is_admin),
        None => Ok(false),
    }
}

fn encode_base64(v: &[u8]) -> String {
    base64_engine.encode(v)
}

pub(crate) fn hash_password(salt: &str, password: &str) -> String {
    let mut hasher = Sha3_256::new();
    hasher.update(salt.as_bytes());
    hasher.update(password.as_bytes());
    format!("{:064x}", hasher.finalize())
}
