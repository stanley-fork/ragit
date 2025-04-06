use crate::error::Error;
use sqlx::postgres::PgPool;

pub enum PushResult {
    Completed,
    Failed,
}

impl PushResult {
    pub fn as_str(&self) -> &'static str {
        match self {
            PushResult::Completed => "completed",
            PushResult::Failed => "failed",
        }
    }
}

pub async fn create_new_session(repo_id: i32, pool: &PgPool) -> Result<String, Error> {
    let curr_sessions = sqlx::query!(
        "SELECT id FROM push_session WHERE repo_id = $1 AND session_state = 'going' LIMIT 1;",
        repo_id,
    ).fetch_all(pool).await?;

    if curr_sessions.len() > 0 {
        // TODO: is it even an error?
        // TODO: it cannot perfectly prevent data-race
        return Err(Error::ServerBusy);
    }

    let session_id = format!(
        "{:032x}{:032x}",
        rand::random::<u128>(),
        rand::random::<u128>(),
    );

    sqlx::query!(
        "INSERT
        INTO push_session (id, repo_id, session_state, updated_at)
        VALUES ($1, $2, 'going', NOW())",
        session_id,
        repo_id,
    ).execute(pool).await?;
    Ok(session_id)
}

pub async fn add_archive(session_id: &str, archive_id: &str, archive: &[u8], pool: &PgPool) -> Result<(), Error> {
    let curr_session = sqlx::query!(
        "UPDATE push_session SET updated_at = NOW() WHERE id = $1",
        session_id,
    ).execute(pool).await?.rows_affected();

    if curr_session == 0 {
        return Err(Error::NoSuchSession(session_id.to_string()));
    }

    let blob_id = format!(
        "{:032x}{:032x}",
        rand::random::<u128>(),
        rand::random::<u128>(),
    );
    sqlx::query!(
        "INSERT INTO push_blob (id, blob) VALUES ($1, $2)",
        blob_id,
        archive,
    ).execute(pool).await?;

    sqlx::query!(
        "INSERT
        INTO push_archive (session_id, archive_id, blob_size, blob_id)
        VALUES ($1, $2, $3, $4)",
        session_id,
        archive_id,
        archive.len() as i32,
        blob_id,
    ).execute(pool).await?;
    Ok(())
}

pub async fn finalize_push(
    repo_id: i32,
    session_id: &str,
    result: PushResult,
    pool: &PgPool,
) -> Result<(), Error> {
    sqlx::query!(
        "UPDATE push_session SET session_state = $1, updated_at = NOW() WHERE id = $2",
        result.as_str(),
        session_id,
    ).execute(pool).await?;
    sqlx::query!(
        "UPDATE repository
        SET
            repo_size = (SELECT SUM(blob_size) FROM push_archive WHERE session_id = $1),
            push_session_id = $2,
            pushed_at = NOW(),
            updated_at = NOW()
        WHERE id = $3",
        session_id,
        session_id,
        repo_id,
    ).execute(pool).await?;

    // TODO: update `push_clone` table

    Ok(())
}
