use chrono::{DateTime, Utc};
use chrono::serde::ts_milliseconds;
use crate::error::Error;
use ragit::{MultiTurnSchema, QueryResponse};
use serde::Serialize;
use sqlx::postgres::PgPool;

// `chat` table
#[derive(Clone, Debug, Serialize)]
pub struct Chat {
    pub id: i32,
    pub repo_id: i32,
    pub title: Option<String>,

    #[serde(with = "ts_milliseconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ts_milliseconds")]
    pub updated_at: DateTime<Utc>,
}

// One that FrontEnd can render
#[derive(Clone, Debug, Serialize)]
pub struct ChatWithHistory {
    pub id: i32,
    pub repo_id: i32,
    pub title: Option<String>,

    #[serde(with = "ts_milliseconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ts_milliseconds")]
    pub updated_at: DateTime<Utc>,
    pub history: Vec<ChatHistory>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ChatHistory {
    pub query: String,
    pub response: String,
    pub model: String,
    pub chunk_uids: Vec<String>,
    pub multi_turn_schema: Option<MultiTurnSchema>,

    #[serde(with = "ts_milliseconds")]
    pub created_at: DateTime<Utc>,
}

pub async fn create_and_return_id(repo_id: i32, pool: &PgPool) -> Result<i32, Error> {
    let row = sqlx::query!(
        "INSERT INTO chat (repo_id, title, created_at, updated_at) VALUES ($1, NULL, NOW(), NOW()) RETURNING id",
        repo_id,
    ).fetch_one(pool).await?;

    Ok(row.id)
}

pub async fn get_chat_by_id(id: i32, pool: &PgPool) -> Result<Chat, Error> {
    let row = sqlx::query!(
        "SELECT id, repo_id, title, created_at, updated_at FROM chat WHERE id = $1",
        id,
    ).fetch_one(pool).await?;

    Ok(Chat {
        id: row.id,
        repo_id: row.repo_id,
        title: row.title.clone(),
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

pub async fn get_chat_with_history_by_id(id: i32, pool: &PgPool) -> Result<ChatWithHistory, Error> {
    let chat = get_chat_by_id(id, pool).await?;
    let history = get_history_by_id(id, pool).await?;

    Ok(ChatWithHistory {
        id: chat.id,
        repo_id: chat.repo_id,
        title: chat.title.clone(),
        created_at: chat.created_at,
        updated_at: chat.updated_at,
        history,
    })
}

pub async fn get_list_by_repo_id(
    repo_id: i32,
    limit: i64,
    offset: i64,
    pool: &PgPool,
) -> Result<Vec<Chat>, Error> {
    let rows = sqlx::query!(
        "SELECT id, repo_id, title, created_at, updated_at FROM chat WHERE repo_id = $1 LIMIT $2 OFFSET $3",
        repo_id,
        limit,
        offset,
    ).fetch_all(pool).await?;
    let mut result = vec![];

    for row in rows.iter() {
        result.push(Chat {
            id: row.id,
            repo_id: row.repo_id,
            title: row.title.clone(),
            created_at: row.created_at,
            updated_at: row.updated_at,
        });
    }

    Ok(result)
}

pub async fn add_chat_history(
    chat_id: i32,
    query: &str,
    history: &[ChatHistory],
    response: &QueryResponse,
    user_id: i32,
    model: &str,
    pool: &PgPool,
) -> Result<(), Error> {
    let now = chrono::offset::Utc::now();
    sqlx::query!(
        "UPDATE chat SET updated_at = $1 WHERE id = $2",
        now,
        chat_id,
    ).execute(pool).await?;

    let multi_turn_schema = match &response.multi_turn_schema {
        Some(m) => Some(serde_json::to_string(m)?),
        None => None,
    };

    let chat_history_id = sqlx::query!(
        "INSERT
        INTO chat_history (chat_id, turn, user_id, model, query, response, multi_turn_schema, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id",
        chat_id,
        history.len() as i32,
        user_id,
        model,
        query,
        &response.response,
        multi_turn_schema,
        now,
    ).fetch_one(pool).await?.id;

    for (index, chunk) in response.retrieved_chunks.iter().enumerate() {
        sqlx::query!(
            "INSERT
            INTO chat_history_chunk_uid (chat_history_id, seq, chunk_uid)
            VALUES ($1, $2, $3)",
            chat_history_id,
            index as i32,
            chunk.uid.to_string(),
        ).execute(pool).await?;
    }

    Ok(())
}

pub async fn get_history_by_id(chat_id: i32, pool: &PgPool) -> Result<Vec<ChatHistory>, Error> {
    let rows = sqlx::query!(
        "SELECT id, query, response, model, multi_turn_schema, created_at FROM chat_history WHERE chat_id = $1 ORDER BY turn",
        chat_id,
    ).fetch_all(pool).await?;
    let mut history = vec![];

    for row in rows.iter() {
        let multi_turn_schema = if let Some(multi_turn_schema) = &row.multi_turn_schema {
            Some(serde_json::from_str(&multi_turn_schema)?)
        } else {
            None
        };
        let chunk_uids = sqlx::query!(
            "SELECT chunk_uid FROM chat_history_chunk_uid WHERE chat_history_id = $1 ORDER BY seq",
            row.id,
        ).fetch_all(pool).await?.into_iter().map(|row| row.chunk_uid).collect();

        history.push(ChatHistory {
            query: row.query.to_string(),
            response: row.response.to_string(),
            model: row.model.to_string(),
            multi_turn_schema,
            chunk_uids,
            created_at: row.created_at,
        });
    }

    Ok(history)
}
