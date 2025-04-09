use crate::error::Error;
use chrono::Datelike;
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

pub async fn get_list(session_id: &str, pool: &PgPool) -> Result<Vec<String>, Error> {
    let rows = crate::query!(
        "SELECT archive_id FROM archive WHERE session_id = $1",
        session_id,
    ).fetch_all(pool).await?;

    Ok(rows.into_iter().map(|row| row.archive_id).collect())
}

pub async fn get_archive(session_id: &str, archive_id: &str, pool: &PgPool) -> Result<Vec<u8>, Error> {
    let blob = crate::query!(
        "SELECT blob
        FROM archive JOIN archive_blob ON archive_blob.id = archive.blob_id
        WHERE session_id = $1 AND archive_id = $2",
        session_id,
        archive_id,
    ).fetch_one(pool).await?;

    match blob.blob {
        Some(blob) => Ok(blob),
        None => Err(Error::ArchiveBlobRemoved),
    }
}

pub async fn create_new_session(repo_id: i32, pool: &PgPool) -> Result<String, Error> {
    let curr_sessions = crate::query!(
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

    crate::query!(
        "INSERT
        INTO push_session (id, repo_id, session_state, updated_at)
        VALUES ($1, $2, 'going', NOW())",
        &session_id,
        repo_id,
    ).execute(pool).await?;
    Ok(session_id)
}

pub async fn add_archive(session_id: &str, archive_id: &str, archive: &[u8], pool: &PgPool) -> Result<(), Error> {
    let curr_session = crate::query!(
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
    crate::query!(
        "INSERT INTO archive_blob (id, blob) VALUES ($1, $2)",
        &blob_id,
        archive,
    ).execute(pool).await?;

    crate::query!(
        "INSERT
        INTO archive (session_id, archive_id, blob_size, blob_id, created_at)
        VALUES ($1, $2, $3, $4, NOW())",
        session_id,
        archive_id,
        archive.len() as i32,
        &blob_id,
    ).execute(pool).await?;
    Ok(())
}

pub async fn finalize_push(
    repo_id: i32,
    session_id: &str,
    result: PushResult,
    pool: &PgPool,
) -> Result<(), Error> {
    let now = chrono::Utc::now();
    let (year, month, day) = (now.year(), now.month(), now.day());

    crate::query!(
        "UPDATE push_session SET session_state = $1, updated_at = NOW() WHERE id = $2",
        result.as_str(),
        session_id,
    ).execute(pool).await?;
    crate::query!(
        "UPDATE repository
        SET
            repo_size = (SELECT SUM(blob_size) FROM archive WHERE session_id = $1),
            push_session_id = $2,
            pushed_at = NOW(),
            updated_at = NOW()
        WHERE id = $3",
        session_id,
        session_id,
        repo_id,
    ).execute(pool).await?;
    crate::query!(
        "INSERT
        INTO repository_stat (repo_id, date_str, year, month, day, push, clone)
        VALUES ($1, $2, $3, $4, $5, 1, 0)
        ON CONFLICT (repo_id, date_str)
        DO UPDATE SET push = repository_stat.push + 1;",
        repo_id,
        format!("{year:04}-{month:02}-{day:02}"),
        year,
        month as i32,
        day as i32,
    ).execute(pool).await?;

    Ok(())
}

pub async fn increment_clone_count(repo_id: i32, pool: &PgPool) -> Result<(), Error> {
    let now = chrono::Utc::now();
    let (year, month, day) = (now.year(), now.month(), now.day());

    crate::query!(
        "INSERT
        INTO repository_stat (repo_id, date_str, year, month, day, push, clone)
        VALUES ($1, $2, $3, $4, $5, 0, 1)
        ON CONFLICT (repo_id, date_str)
        DO UPDATE SET clone = repository_stat.clone + 1;",
        repo_id,
        format!("{year:04}-{month:02}-{day:02}"),
        year,
        month as i32,
        day as i32,
    ).execute(pool).await?;
    Ok(())
}

pub async fn is_first_archive(session_id: &str, archive_id: &str, pool: &PgPool) -> Result<bool, Error> {
    let maybe_row = crate::query!(
        "SELECT archive_id FROM archive WHERE session_id = $1 ORDER BY created_at DESC LIMIT 1",
        session_id,
    ).fetch_all(pool).await?;

    match maybe_row.get(0) {
        Some(row) if row.archive_id == archive_id => Ok(true),
        _ => Ok(false),
    }
}
