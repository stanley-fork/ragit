use crate::error::Error;
use crate::utils::get_rag_path;
use ragit_api::ModelRaw;
use ragit_fs::{
    WriteMode,
    join,
    write_string,
};
use sqlx::postgres::PgPool;

pub async fn get_default_model_name(user_id: i32, pool: &PgPool) -> Result<String, Error> {
    let name = sqlx::query!(
        "SELECT ai_model.name
        FROM ai_model JOIN user_ai_model ON ai_model.id = user_ai_model.ai_model_id
        WHERE user_ai_model.default_model = TRUE AND user_ai_model.user_id = $1",
        user_id,
    ).fetch_one(pool).await?.name;
    Ok(name)
}

pub async fn get_model_schema(user_id: i32, model_name: &str, pool: &PgPool) -> Result<ModelRaw, Error> {
    let row = sqlx::query!(
        "SELECT
            ai_model.name,
            ai_model.api_name,
            ai_model.api_provider,
            ai_model.api_url,
            ai_model.can_read_images,
            user_ai_model.api_key
        FROM ai_model JOIN user_ai_model ON ai_model.id = user_ai_model.ai_model_id
        WHERE user_ai_model.user_id = $1 AND ai_model.name = $2",
        user_id,
        model_name,
    ).fetch_one(pool).await?;

    Ok(ModelRaw {
        name: row.name.clone(),
        api_name: row.api_name.clone(),
        can_read_images: row.can_read_images,
        api_provider: row.api_provider.clone(),
        api_url: row.api_url.clone(),
        api_key: row.api_key.clone(),

        // We're not doing this with ragit-server
        input_price: 0.0,
        output_price: 0.0,
        explanation: None,
        api_env_var: None,

        // TODO: make it configurable?
        api_timeout: None,
    })
}

pub fn update_model_schema(user: &str, repo: &str, schema: &ModelRaw) -> Result<(), Error> {
    let rag_path = get_rag_path(user, repo)?;

    write_string(
        &join(&rag_path, "models.json")?,
        &serde_json::to_string(&[schema])?,
        WriteMode::CreateOrTruncate,
    )?;

    Ok(())
}
