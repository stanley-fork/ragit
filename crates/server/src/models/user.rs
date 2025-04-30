use super::auth;
use chrono::{DateTime, Utc};
use chrono::serde::{ts_milliseconds, ts_milliseconds_option};
use crate::error::Error;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserDetail {
    pub id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub readme: Option<String>,
    pub public: bool,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserSimple {
    pub id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub public: bool,
    #[serde(with = "ts_milliseconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ts_milliseconds_option")]
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserCreate {
    pub id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub password: String,
    pub readme: Option<String>,
    pub public: bool,
}

pub async fn get_list(include_privates: bool, limit: i64, offset: i64, pool: &PgPool) -> Result<Vec<UserSimple>, Error> {
    // TODO: how do I parameterize `query!` macro?
    if include_privates {
        let rows = crate::query_as!(
            UserSimple,
            "SELECT id, name, email, public, created_at, last_login_at FROM user_ ORDER BY id LIMIT $1 OFFSET $2",
            limit,
            offset,
        ).fetch_all(pool).await?;

        Ok(rows)
    }

    else {
        let rows = crate::query_as!(
            UserSimple,
            "SELECT id, name, email, public, created_at, last_login_at FROM user_ WHERE public = TRUE ORDER BY id LIMIT $1 OFFSET $2",
            limit,
            offset,
        ).fetch_all(pool).await?;

        Ok(rows)
    }
}

pub async fn get_detail(id: &str, pool: &PgPool) -> Result<UserDetail, Error> {
    let result = crate::query_as!(
        UserDetail,
        "SELECT id, name, email, public, readme, created_at, last_login_at FROM user_ WHERE id = $1",
        id,
    ).fetch_one(pool).await?;

    Ok(result)
}

pub async fn create(user: &UserCreate, pool: &PgPool) -> Result<(), Error> {
    let salt = format!("{:x}", rand::random::<u128>());
    let password = auth::hash_password(&salt, &user.password);

    crate::query!(
        "INSERT
        INTO user_ (
            id,
            name,
            email,
            salt,
            password,
            password_hash_type,
            readme,
            public,
            is_admin,
            created_at,
            last_login_at
        )
        VALUES (
            $1,   -- id
            $2,   -- name
            $3,   -- email
            $4,   -- salt
            $5,   -- password
            'SHA3-256',  -- password_hash_type
            $6,   -- readme
            $7,   -- public
            (SELECT COUNT(*) = 0 FROM user_),  -- is_admin
            NOW(),  -- created_at
            NULL  -- last_login_at
        )",
        user.id.clone(),
        user.name.clone(),
        user.email.clone(),
        salt,
        password,
        user.readme.clone(),
        user.public,
    ).execute(pool).await?;

    Ok(())
}

pub async fn check_auth(user: &str, api_key: Option<String>, pool: &PgPool) -> Result<bool, Error> {
    let permission = auth::get_user_id_from_api_key(api_key, pool).await?;

    match permission {
        Some(auth::Permission { user: user_, is_admin }) if user == user_ || is_admin => Ok(true),
        _ => Ok(false),
    }
}

pub async fn no_user_at_all(pool: &PgPool) -> Result<bool, Error> {
    let rows = crate::query!("SELECT id FROM user_ LIMIT 1").fetch_all(pool).await?;
    Ok(rows.len() == 0)
}
