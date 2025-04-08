use super::get_pool;
use crate::error::Error;

// It's not an api endpoint. It's a cli command
// TODO: It's not a good idea to hard-code the table names.
pub async fn drop_all() -> Result<(), Error> {
    let pool = get_pool().await;
    sqlx::query!("DROP TABLE ai_model").execute(pool).await?;
    sqlx::query!("DROP TABLE archive").execute(pool).await?;
    sqlx::query!("DROP TABLE archive_blob").execute(pool).await?;
    sqlx::query!("DROP TABLE chat").execute(pool).await?;
    sqlx::query!("DROP TABLE chat_history").execute(pool).await?;
    sqlx::query!("DROP TABLE chat_history_chunk_uid").execute(pool).await?;
    sqlx::query!("DROP TABLE issue").execute(pool).await?;
    sqlx::query!("DROP TABLE issue_comment").execute(pool).await?;
    sqlx::query!("DROP TABLE issue_comment_content_history").execute(pool).await?;
    sqlx::query!("DROP TABLE issue_content_history").execute(pool).await?;
    sqlx::query!("DROP TABLE push_session").execute(pool).await?;
    sqlx::query!("DROP TABLE repository").execute(pool).await?;
    sqlx::query!("DROP TABLE repository_stat").execute(pool).await?;
    sqlx::query!("DROP TABLE user_").execute(pool).await?;
    sqlx::query!("DROP TABLE user_ai_model").execute(pool).await?;
    sqlx::query!("TRUNCATE _sqlx_migrations").execute(pool).await?;  // so that you can run `sqlx migrate run` again
    Ok(())
}

// It's not an api endpoint. It's a cli command
// TODO: It's not a good idea to hard-code the table names.
pub async fn truncate_all() -> Result<(), Error> {
    let pool = get_pool().await;
    sqlx::query!("TRUNCATE ai_model").execute(pool).await?;
    sqlx::query!("TRUNCATE archive").execute(pool).await?;
    sqlx::query!("TRUNCATE archive_blob").execute(pool).await?;
    sqlx::query!("TRUNCATE chat").execute(pool).await?;
    sqlx::query!("TRUNCATE chat_history").execute(pool).await?;
    sqlx::query!("TRUNCATE chat_history_chunk_uid").execute(pool).await?;
    sqlx::query!("TRUNCATE issue").execute(pool).await?;
    sqlx::query!("TRUNCATE issue_comment").execute(pool).await?;
    sqlx::query!("TRUNCATE issue_comment_content_history").execute(pool).await?;
    sqlx::query!("TRUNCATE issue_content_history").execute(pool).await?;
    sqlx::query!("TRUNCATE push_session").execute(pool).await?;
    sqlx::query!("TRUNCATE repository").execute(pool).await?;
    sqlx::query!("TRUNCATE repository_stat").execute(pool).await?;
    sqlx::query!("TRUNCATE user_").execute(pool).await?;
    sqlx::query!("TRUNCATE user_ai_model").execute(pool).await?;
    Ok(())
}
