use super::auth::hash_password;
use chrono::{DateTime, Utc};
use chrono::serde::ts_milliseconds;
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
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserAiModel {
    pub id: String,
    pub name: String,
    pub api_name: String,
    pub api_provider: String,
    pub api_url: Option<String>,
    pub can_read_images: bool,
    pub input_price: f64,
    pub output_price: f64,
    pub explanation: Option<String>,
    pub api_env_var: Option<String>,
    pub api_key_preview: Option<String>,
    pub default_model: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserAiModelUpdate {
    pub model_id: String,
    pub default_model: bool,
    pub api_key: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AiModel {
    pub id: String,
    pub name: String,
    pub api_name: String,
    pub api_provider: String,
    pub api_url: Option<String>,
    pub can_read_images: bool,
    pub input_price: f64,
    pub output_price: f64,
    pub explanation: Option<String>,
    pub api_env_var: Option<String>,
    pub tags: Vec<String>,

    #[serde(with = "ts_milliseconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ts_milliseconds")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AiModelCreation {
    pub name: String,
    pub api_name: String,
    pub api_provider: String,
    pub api_url: Option<String>,
    pub can_read_images: bool,
    pub input_price: f64,
    pub output_price: f64,
    pub explanation: Option<String>,
    pub api_env_var: Option<String>,
    pub tags: Vec<String>,
}

impl From<&AiModel> for ragit_api::ModelRaw {
    fn from(m: &AiModel) -> ragit_api::ModelRaw {
        ragit_api::ModelRaw {
            name: m.name.clone(),
            api_name: m.api_name.clone(),
            api_provider: m.api_provider.clone(),
            api_url: m.api_url.clone(),
            can_read_images: m.can_read_images,
            input_price: m.input_price,
            output_price: m.output_price,
            explanation: m.explanation.clone(),
            api_env_var: m.api_env_var.clone(),
            api_key: None,
            api_timeout: None,
        }
    }
}

pub async fn upsert_and_return_id(model: &AiModelCreation, pool: &PgPool) -> Result<String, Error> {
    let model_id = hash_model(model);
    crate::query!(
        "INSERT
        INTO ai_model (
            id,
            name,
            api_name,
            api_provider,
            api_url,
            can_read_images,
            input_price,
            output_price,
            explanation,
            api_env_var,
            tags,
            created_at,
            updated_at
        )
        VALUES (
            $1,   -- id
            $2,   -- name
            $3,   -- api_name
            $4,   -- api_provider
            $5,   -- api_url
            $6,   -- can_read_images
            $7,   -- input_price
            $8,   -- output_price
            $9,   -- explanation
            $10,  -- api_env_var
            $11,  -- tags
            NOW(),  -- created_at
            NOW()   -- updated_at
        )
        ON CONFLICT (id)
        DO UPDATE SET
            -- if `id` is the same, $2 ~ $5 must be the same
            can_read_images = $6,
            input_price = $7,
            output_price = $8,
            explanation = $9,
            api_env_var = $10,
            updated_at = NOW()",
        &model_id,
        &model.name,
        &model.api_name,
        &model.api_provider,
        model.api_url.as_ref().map(|s| s.as_str()),  // `Option<&str>` works, but `&Option<String>` does not... why?
        model.can_read_images,
        model.input_price,
        model.output_price,
        model.explanation.as_ref().map(|s| s.as_str()),
        model.api_env_var.as_ref().map(|s| s.as_str()),
        &model.tags.clone(),
    ).execute(pool).await?;

    Ok(model_id)
}

pub async fn get_list(
    name: Option<String>,
    tags: Vec<String>,
    limit: i64,
    offset: i64,
    pool: &PgPool,
) -> Result<Vec<AiModel>, Error> {
    // TODO: how can I use sqlx as a sql builder?
    match (&name, tags.len()) {
        (None, 0) | (Some(_), _) => {
            let mut models = crate::query_as!(
                AiModel,
                "SELECT
                    id,
                    name,
                    api_name,
                    api_provider,
                    api_url,
                    can_read_images,
                    input_price,
                    output_price,
                    explanation,
                    api_env_var,
                    tags,
                    created_at,
                    updated_at
                FROM ai_model
                ORDER BY id
                LIMIT $1
                OFFSET $2",
                limit,
                offset,
            ).fetch_all(pool).await?;

            // It does an unnecessary roundtrips to `ragit_api::ModelRaw` and `ragit_api::Model`
            // because I want to use `ragit_api::get_model_by_name` so that the api behaves
            // exactly the same as `rag ls-models`.
            if let Some(name) = &name {
                let model_by_name = models.iter().map(|model| (model.name.clone(), model.clone())).collect::<HashMap<_, _>>();
                let ra_models = models.iter().filter_map(
                    |model| match tags.len() {
                        0 => ragit_api::Model::try_from(&ragit_api::ModelRaw::from(model)).ok(),
                        _ => if tags.iter().all(|tag| model.tags.contains(tag)) {
                            ragit_api::Model::try_from(&ragit_api::ModelRaw::from(model)).ok()
                        } else {
                            None
                        },
                    }
                ).collect::<Vec<_>>();
                let model_names = match ragit_api::get_model_by_name(&ra_models, name) {
                    Ok(model) => vec![model.name.clone()],
                    Err(ragit_api::Error::InvalidModelName { candidates, .. }) => candidates,
                    _ => vec![],
                };

                models = model_names.iter().map(|name| model_by_name.get(name).unwrap().clone()).collect()
            }

            Ok(models)
        },
        (None, _) => {
            let mut models = crate::query_as!(
                AiModel,
                "SELECT
                    id,
                    name,
                    api_name,
                    api_provider,
                    api_url,
                    can_read_images,
                    input_price,
                    output_price,
                    explanation,
                    api_env_var,
                    tags,
                    created_at,
                    updated_at
                FROM ai_model
                WHERE $1 = ANY(tags)
                ORDER BY id
                LIMIT $2
                OFFSET $3",
                &tags[0],
                limit,
                offset,
            ).fetch_all(pool).await?;

            // TODO: how do I query multiple tags against multiple tags using SQL?
            if tags.len() > 1 {
                models = models.into_iter().filter(
                    |model| tags.iter().all(|tag| model.tags.contains(tag))
                ).collect();
            }

            Ok(models)
        },
    }
}

pub async fn get_list_by_user_id(user: &str, pool: &PgPool) -> Result<Vec<UserAiModel>, Error> {
    let rows = crate::query!(
        "SELECT
            ai_model.id,
            ai_model.name,
            ai_model.api_name,
            ai_model.api_provider,
            ai_model.api_url,
            ai_model.can_read_images,
            ai_model.input_price,
            ai_model.output_price,
            ai_model.explanation,
            ai_model.api_env_var,
            user_ai_model.api_key,
            user_ai_model.default_model
        FROM user_ai_model JOIN ai_model ON user_ai_model.ai_model_id = ai_model.id
        WHERE user_ai_model.user_ = $1",
        user,
    ).fetch_all(pool).await?;
    let mut result = Vec::with_capacity(rows.len());

    for row in rows.iter() {
        result.push(UserAiModel {
            id: row.id.clone(),
            name: row.name.clone(),
            api_name: row.api_name.clone(),
            api_provider: row.api_provider.clone(),
            api_url: row.api_url.clone(),
            can_read_images: row.can_read_images,
            input_price: row.input_price,
            output_price: row.output_price,
            explanation: row.explanation.clone(),
            api_env_var: row.api_env_var.clone(),
            api_key_preview: row.api_key.as_ref().map(|key| hide_api_key(key)),
            default_model: row.default_model,
        });
    }

    Ok(result)
}

// TODO: It has a big problem.
//       A user always have to send a complete `UserAiModelUpdate`, which
//       includes `api_key` field. But the user can never know the api key
//       because `GET /user-list/{user}/ai-model-list` only returns a preview
//       of the api key.
pub async fn register(
    user: &str,
    update: &UserAiModelUpdate,
    pool: &PgPool,
) -> Result<(), Error> {
    crate::query!(
        "INSERT
        INTO user_ai_model (user_, ai_model_id, api_key, default_model, added_at)
        VALUES ($1, $2, $3, $4, NOW())
        ON CONFLICT (user_, ai_model_id)
        DO UPDATE SET api_key = $3, default_model = $4;",
        user,
        &update.model_id,
        update.api_key.as_ref().map(|s| s.as_str()),
        update.default_model,
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
            ai_model.input_price,
            ai_model.output_price,
            ai_model.explanation,
            ai_model.api_env_var,
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
        input_price: row.input_price,
        output_price: row.output_price,
        explanation: row.explanation.clone(),
        api_env_var: row.api_env_var.clone(),

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

/// It initializes models only if there's no model at all (a.k.a the server is run for the first time).
pub async fn initialize_ai_models(pool: &PgPool) -> Result<(), Error> {
    let j = include_str!("../../../../models.json");
    let models = serde_json::from_str::<Vec<AiModelCreation>>(j)?;

    for model in models.iter() {
        upsert_and_return_id(model, pool).await?;
    }

    Ok(())
}

pub fn update_model_schema_on_disk(user: &str, repo: &str, schema: &ModelRaw) -> Result<(), Error> {
    let rag_path = get_rag_path(user, repo)?;

    write_string(
        &join(&rag_path, "models.json")?,
        &serde_json::to_string(&[schema])?,
        WriteMode::CreateOrTruncate,
    )?;

    Ok(())
}

fn hash_model(m: &AiModelCreation) -> String {
    let s = format!(
        "{}\n{}\n{}\n{}",
        m.name,
        m.api_name,
        m.api_provider,
        m.api_url.as_ref().map(|s| s.as_str()).unwrap_or("null"),
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
