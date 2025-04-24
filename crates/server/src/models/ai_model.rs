use super::auth::hash_password;
use crate::error::Error;
use crate::utils::{get_rag_path, trim_long_string};
use ragit_api::ModelRaw;
use ragit_fs::{
    WriteMode,
    join,
    write_string,
};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AiModel {
    pub id: String,
    pub name: String,
    pub api_name: String,
    pub api_provider: String,
    pub api_url: Option<String>,
    pub can_read_images: bool,
    pub api_key_preview: Option<String>,
    pub default_model: bool,
}

pub async fn create_and_return_id(model: &ModelRaw, pool: &PgPool) -> Result<String, Error> {
    let model_id = hash_model(model);
    crate::query!(
        "INSERT
        INTO ai_model (id, name, api_name, api_provider, api_url, can_read_images)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (id) DO NOTHING",
        &model_id,
        &model.name,
        &model.api_name,
        &model.api_provider,
        model.api_url.as_ref().map(|s| s.as_str()),  // `Option<&str>` works, but `&Option<String>` does not... why?
        model.can_read_images,
    ).execute(pool).await?;

    Ok(model_id)
}

pub async fn get_list_by_user_id(user: &str, pool: &PgPool) -> Result<Vec<AiModel>, Error> {
    let rows = crate::query!(
        "SELECT
            ai_model.id,
            ai_model.name,
            ai_model.api_name,
            ai_model.api_provider,
            ai_model.api_url,
            ai_model.can_read_images,
            user_ai_model.api_key,
            user_ai_model.default_model
        FROM user_ai_model JOIN ai_model ON user_ai_model.ai_model_id = ai_model.id
        WHERE user_ai_model.user_ = $1",
        user,
    ).fetch_all(pool).await?;
    let mut result = Vec::with_capacity(rows.len());

    for row in rows.iter() {
        result.push(AiModel {
            id: row.id.clone(),
            name: row.name.clone(),
            api_name: row.api_name.clone(),
            api_provider: row.api_provider.clone(),
            api_url: row.api_url.clone(),
            can_read_images: row.can_read_images,
            api_key_preview: row.api_key.as_ref().map(|key| hide_api_key(key)),
            default_model: row.default_model,
        });
    }

    Ok(result)
}

pub async fn register(
    user: &str,
    model_id: &str,
    api_key: Option<String>,
    default_model: bool,
    pool: &PgPool,
) -> Result<(), Error> {
    crate::query!(
        "INSERT
        INTO user_ai_model (user_, ai_model_id, api_key, default_model, added_at)
        VALUES ($1, $2, $3, $4, NOW())",
        user,
        model_id,
        api_key.as_ref().map(|s| s.as_str()),
        default_model,
    ).execute(pool).await?;
    Ok(())
}

pub async fn get_default_model_name(user: &str, pool: &PgPool) -> Result<String, Error> {
    let name = crate::query!(
        "SELECT ai_model.name
        FROM ai_model JOIN user_ai_model ON ai_model.id = user_ai_model.ai_model_id
        WHERE user_ai_model.default_model = TRUE AND user_ai_model.user_ = $1",
        user,
    ).fetch_one(pool).await?.name;
    Ok(name)
}

pub async fn get_model_schema(user: &str, model_name: &str, pool: &PgPool) -> Result<ModelRaw, Error> {
    let row = crate::query!(
        "SELECT
            ai_model.name,
            ai_model.api_name,
            ai_model.api_provider,
            ai_model.api_url,
            ai_model.can_read_images,
            user_ai_model.api_key
        FROM ai_model JOIN user_ai_model ON ai_model.id = user_ai_model.ai_model_id
        WHERE user_ai_model.user_ = $1 AND ai_model.name = $2",
        user,
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

pub async fn update_api_key(user: &str, model: &str, api_key: Option<String>, pool: &PgPool) -> Result<(), Error> {
    // I'm querying twice because
    //    1. I don't know how to use JOIN with an UPDATE clause.
    //    2. `fetch_one` makes sure that a row exists
    let model_id = crate::query!(
        "SELECT ai_model.id
        FROM ai_model JOIN user_ai_model ON user_ai_model.ai_model_id = ai_model.id
        WHERE user_ai_model.user_ = $1 AND ai_model.name = $2",
        user,
        model,
    ).fetch_one(pool).await?.id;
    crate::query!(
        "UPDATE user_ai_model SET api_key = $1 WHERE user_ = $2 AND ai_model_id = $3",
        api_key.as_ref().map(|s| s.as_str()),
        user,
        model_id,
    ).execute(pool).await?;

    Ok(())
}

pub async fn set_default_model(user: &str, model: &str, pool: &PgPool) -> Result<(), Error> {
    let model_id = crate::query!(
        "SELECT ai_model.id
        FROM ai_model JOIN user_ai_model ON user_ai_model.ai_model_id = ai_model.id
        WHERE user_ai_model.user_ = $1 AND ai_model.name = $2",
        user,
        model,
    ).fetch_one(pool).await?.id;
    crate::query!(
        "UPDATE user_ai_model SET default_model = (ai_model_id = $1) WHERE user_ = $2",
        model_id,
        user,
    ).execute(pool).await?;

    Ok(())
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

fn hash_model(m: &ModelRaw) -> String {
    let s = format!(
        "{}\n{}\n{}\n{}\n{}",
        m.name,
        m.api_name,
        m.api_provider,
        m.api_url.as_ref().map(|s| s.as_str()).unwrap_or("null"),
        m.can_read_images,
    );

    hash_password("", &s)
}

fn hide_api_key(key: &str) -> String {
    if key.chars().count() > 12 {
        trim_long_string(key, 4, 4)
    }

    else {
        String::from("...")
    }
}
