use chrono::{DateTime, Utc};
use chrono::serde::{ts_milliseconds, ts_milliseconds_option};
use crate::error::Error;
use crate::utils::normalize;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;

pub enum PasswordHashType {
    Dummy,
}

impl PasswordHashType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PasswordHashType::Dummy => "dummy",
        }
    }
}

#[derive(Clone, Debug)]
pub struct UserDetail {
    pub id: i32,
    pub name: String,
    pub normalized_name: String,
    pub email: Option<String>,
    pub readme: String,
    pub created_at: DateTime<Utc>,
    pub last_login_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct UserSimple {
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
    let rows = sqlx::query!(
        "SELECT name, normalized_name, email, created_at, last_login_at FROM user_ ORDER BY id LIMIT $1 OFFSET $2",
        limit,
        offset,
    ).fetch_all(pool).await?;
    let mut result = Vec::with_capacity(rows.len());

    for row in rows.iter() {
        result.push(UserSimple {
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
    let row = sqlx::query!(
        "SELECT id FROM user_ WHERE normalized_name = $1",
        name,
    ).fetch_one(pool).await?;

    Ok(row.id)
}

pub async fn create_and_return_id(user: &UserCreate, pool: &PgPool) -> Result<i32, Error> {
    let salt = format!("{:032x}", rand::random::<u128>());
    let (password, password_hash_type) = hash_password(&salt, &user.password);
    let user_id = sqlx::query!(
        "INSERT
        INTO user_ (
            name,
            normalized_name,
            email,
            salt,
            password,
            password_hash_type,
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
            $6,   -- password_hash_type
            $7,   -- readme
            $8,   -- public
            NOW(),  -- created_at
            NULL  -- last_login_at
        )
        RETURNING id",
        user.name.clone(),
        normalize(&user.name),
        user.email.clone(),
        salt,
        password,
        password_hash_type.as_str(),
        user.readme.clone(),
        user.public,
    ).fetch_one(pool).await?.id;

    Ok(user_id)
}

// TODO: impl REAL hash function
fn hash_password(salt: &str, password: &str) -> (String, PasswordHashType) {
    let mut state: u128 = 0;

    for (i, b) in salt.bytes().chain(password.bytes()).enumerate() {
        let n = ((i as u128) << 12) | b as u128;
        let n = (((state >> 24) & 0xfff) << 16) + n;
        let n = 2 * n * n + n + 1;
        state += n;
    }

    (format!("{state:x}"), PasswordHashType::Dummy)
}
