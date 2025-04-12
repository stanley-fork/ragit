use chrono::{DateTime, Utc};
use crate::error::Error;
use ragit_pdl::encode_base64;
use sqlx::postgres::PgPool;

pub struct Permission {
    pub user_id: i32,
    pub is_admin: bool,
}

pub async fn get_user_id_from_api_key(api_key: Option<String>, pool: &PgPool) -> Result<Option<Permission>, Error> {
    let Some(api_key) = api_key else { return Ok(None); };
    let rows = crate::query!(
        "SELECT user_id, is_admin FROM api_key JOIN user_ ON api_key.user_id = user_.id WHERE api_key = $1 AND expire > NOW()",
        api_key,
    ).fetch_all(pool).await?;

    match rows.get(0) {
        Some(row) => Ok(Some(Permission { user_id: row.user_id, is_admin: row.is_admin })),
        None => Ok(None),
    }
}

pub async fn create_api_key(user_id: i32, expire: DateTime<Utc>, pool: &PgPool) -> Result<String, Error> {
    let key = format!("rgs_{}", encode_base64(&(0..240).map(|_| rand::random::<u8>()).collect::<Vec<_>>()));
    crate::query!(
        "INSERT INTO api_key (api_key, user_id, expire) VALUES ($1, $2, $3)",
        &key,
        user_id,
        expire,
    ).execute(pool).await?;
    Ok(key)
}

pub async fn is_admin(api_key: Option<String>, pool: &PgPool) -> Result<bool, Error> {
    let Some(api_key) = api_key else { return Ok(false); };
    let rows = crate::query!(
        "SELECT is_admin FROM user_ JOIN api_key ON api_key.user_id = user_.id WHERE api_key.api_key = $1",
        api_key,
    ).fetch_all(pool).await?;

    match rows.get(0) {
        Some(row) => Ok(row.is_admin),
        None => Ok(false),
    }
}
