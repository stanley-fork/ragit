use chrono::{DateTime, Utc};
use chrono::serde::{ts_milliseconds, ts_milliseconds_option};
use crate::error::Error;
use crate::utils::normalize;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use sqlx::postgres::PgPool;

#[derive(Clone, Debug, Serialize)]
pub struct UserDetail {
    pub id: i32,
    pub name: String,
    pub normalized_name: String,
    pub email: Option<String>,
    pub readme: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct UserSimple {
    pub id: i32,
    pub name: String,
    pub normalized_name: String,
    pub email: Option<String>,
    #[serde(with = "ts_milliseconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ts_milliseconds_option")]
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UserCreate {
    pub name: String,
    pub email: Option<String>,
    pub password: String,
    pub readme: Option<String>,
    pub public: bool,
}

pub async fn get_list(limit: i64, offset: i64, pool: &PgPool) -> Result<Vec<UserSimple>, Error> {
    let rows = crate::query!(
        "SELECT id, name, normalized_name, email, created_at, last_login_at FROM user_ ORDER BY id LIMIT $1 OFFSET $2",
        limit,
        offset,
    ).fetch_all(pool).await?;
    let mut result = Vec::with_capacity(rows.len());

    for row in rows.iter() {
        result.push(UserSimple {
            id: row.id,
            name: row.name.clone(),
            normalized_name: row.normalized_name.clone(),
            email: row.email.clone(),
            created_at: row.created_at,
            last_login_at: row.last_login_at,
        });
    }

    Ok(result)
}

pub async fn get_id_by_name(name: &str, pool: &PgPool) -> Result<i32, Error> {
    let name = normalize(name);
    let row = crate::query!(
        "SELECT id FROM user_ WHERE normalized_name = $1",
        name,
    ).fetch_one(pool).await?;

    Ok(row.id)
}

pub async fn get_detail_by_name(name: &str, pool: &PgPool) -> Result<UserDetail, Error> {
    let name = normalize(name);
    let row = crate::query!(
        "SELECT id, name, normalized_name, email, readme, created_at, last_login_at FROM user_ WHERE normalized_name = $1",
        name,
    ).fetch_one(pool).await?;

    Ok(UserDetail {
        id: row.id,
        name: row.name,
        normalized_name: row.normalized_name,
        email: row.email,
        readme: row.readme,
        created_at: row.created_at,
        last_login_at: row.last_login_at,
    })
}

pub async fn create_and_return_id(user: &UserCreate, pool: &PgPool) -> Result<i32, Error> {
    let salt = format!("{:x}", rand::random::<u128>());
    let password = hash_password(&salt, &user.password);

    let user_id = crate::query!(
        "INSERT
        INTO user_ (
            name,
            normalized_name,
            email,
            salt,
            password,
            readme,
            public,
            created_at,
            last_login_at
        )
        VALUES (
            $1,   -- name
            $2,   -- normalized_name
            $3,   -- email
            $4,   -- salt
            $5,   -- password
            $6,   -- readme
            $7,   -- public
            NOW(),  -- created_at
            NULL  -- last_login_at
        )
        RETURNING id",
        user.name.clone(),
        normalize(&user.name),
        user.email.clone(),
        salt,
        password,
        user.readme.clone(),
        user.public,
    ).fetch_one(pool).await?.id;

    Ok(user_id)
}

pub(crate) fn hash_password(salt: &str, password: &str) -> String {
    let mut hasher = Sha3_256::new();
    hasher.update(salt.as_bytes());
    hasher.update(password.as_bytes());
    format!("{:064x}", hasher.finalize())
}
