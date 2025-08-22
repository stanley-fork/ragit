use super::{Index, request_json_file};
use crate::constant::MODEL_FILE_NAME;
use crate::error::Error;
use crate::schema::ModelSchema;
use ragit_api::{
    Model,
    ModelRaw,
    get_model_by_name,
    load_models,
    save_models,
};
use reqwest::Url;
use std::collections::hash_map::{Entry, HashMap};

pub struct FetchModelResult {
    pub fetched: usize,
    pub updated: usize,
}

impl Index {
    pub async fn search_remote_models(keyword: &str, remote: &str) -> Result<Vec<ModelSchema>, Error> {
        let mut remote = remote.to_string();

        if !remote.ends_with("/") {
            remote = format!("{remote}/");
        }

        let url = Url::parse(&remote)?;
        let mut request_url = url.join("ai-model-list/")?;
        request_url.set_query(Some(&format!("name={keyword}")));
        let models = request_json_file(request_url.as_str()).await?;
        let models = serde_json::from_value::<Vec<ModelRaw>>(models)?;
        let mut result = Vec::with_capacity(models.len());

        for model in models.iter() {
            result.push(model.try_into()?);
        }

        Ok(result)
    }

    /// It returns how many models it fetched.
    pub async fn fetch_remote_models(&mut self, name: &str, existing_only: bool, remote: &str) -> Result<FetchModelResult, Error> {
        let mut remote = remote.to_string();

        if !remote.ends_with("/") {
            remote = format!("{remote}/");
        }

        let mut url = Url::parse(&remote)?;
        url = url.join("ai-model-list/")?;
        let remote_models = request_models(url, Some(name)).await?;

        let models_at = Index::get_rag_path(
            &self.root_dir,
            &MODEL_FILE_NAME.to_string(),
        )?;
        let local_models = load_models(&models_at)?;
        let mut local_models = local_models.into_iter().map(
            |model| (model.name.to_string(), model)
        ).collect::<HashMap<String, Model>>();
        let mut fetched = 0;
        let mut updated = 0;

        for remote_model in remote_models.iter() {
            let remote_model: Model = remote_model.try_into()?;

            match local_models.entry(remote_model.name.to_string()) {
                Entry::Occupied(mut local_model) => {
                    if let Some(new_model) = update_model(local_model.get(), &remote_model) {
                        *local_model.get_mut() = new_model;
                        updated += 1;
                    }
                },
                Entry::Vacant(e) => {
                    if existing_only {
                        continue;
                    }

                    e.insert(remote_model);
                    fetched += 1;
                },
            }
        }

        if updated > 0 || fetched > 0 {
            let mut local_models = local_models.into_values().collect::<Vec<_>>();
            local_models.sort_by_key(|m| m.name.to_string());
            save_models(&local_models, &models_at)?;
        }

        Ok(FetchModelResult { updated, fetched })
    }

    /// It returns how many models it fetched.
    pub async fn fetch_all_remote_models(&mut self, existing_only: bool, remote: &str) -> Result<FetchModelResult, Error> {
        let models_at = Index::get_rag_path(
            &self.root_dir,
            &MODEL_FILE_NAME.to_string(),
        )?;
        let local_models = load_models(&models_at)?;
        let mut local_models = local_models.into_iter().map(
            |model| (model.name.to_string(), model)
        ).collect::<HashMap<String, Model>>();

        let mut remote = remote.to_string();

        if !remote.ends_with("/") {
            remote = format!("{remote}/");
        }

        let mut url = Url::parse(&remote)?;
        url = url.join("ai-model-list/")?;

        let remote_models = if existing_only {
            let mut remote_models = vec![];

            for model in local_models.keys() {
                // TODO: there must be a more efficient way to request_json_file
                //       e.g. find a common substring of local models
                url.set_query(Some(&format!("name={model}")));

                let remote_models_ = request_json_file(url.as_str()).await?;
                let mut remote_models_ = serde_json::from_value::<Vec<ModelRaw>>(remote_models_)?;
                remote_models.append(&mut remote_models_);
            }

            remote_models
        } else {
            request_models(url, None).await?
        };
        let mut fetched = 0;
        let mut updated = 0;

        for remote_model in remote_models.iter() {
            let remote_model: Model = remote_model.try_into()?;

            match local_models.entry(remote_model.name.to_string()) {
                Entry::Occupied(mut local_model) => {
                    if let Some(new_model) = update_model(local_model.get(), &remote_model) {
                        *local_model.get_mut() = new_model;
                        updated += 1;
                    }
                },
                Entry::Vacant(e) => {
                    if existing_only {
                        continue;
                    }

                    e.insert(remote_model);
                    fetched += 1;
                },
            }
        }

        if updated > 0 || fetched > 0 {
            let mut local_models = local_models.into_values().collect::<Vec<_>>();
            local_models.sort_by_key(|m| m.name.to_string());
            save_models(&local_models, &models_at)?;
        }

        Ok(FetchModelResult { updated, fetched })
    }

    pub fn remove_local_model(&mut self, name: &str) -> Result<(), Error> {
        let models_at = Index::get_rag_path(
            &self.root_dir,
            &MODEL_FILE_NAME.to_string(),
        )?;
        let mut models = load_models(&models_at)?;
        let target = get_model_by_name(&models, name)?;

        // TODO: what if the user wants to remove models whose names
        //       contain "dummy"?
        if target.is_test_model() {
            return Err(Error::InvalidModelName {
                name: name.to_string(),
                candidates: vec![],
            });
        }

        models = models.into_iter().filter(
            |model| model.name != target.name
        ).collect();
        save_models(&models, &models_at)?;
        Ok(())
    }

    pub fn remove_all_local_models(&mut self) -> Result<(), Error> {
        let models_at = Index::get_rag_path(
            &self.root_dir,
            &MODEL_FILE_NAME.to_string(),
        )?;
        save_models(&[], &models_at)?;
        Ok(())
    }
}

async fn request_models(mut url: Url, name: Option<&str>) -> Result<Vec<ModelRaw>, Error> {
    let mut result = vec![];
    let mut offset = 0;

    loop {
        if let Some(name) = name {
            url.set_query(Some(&format!("name={name}&limit=10&offset={offset}")));
        }

        else {
            url.set_query(Some(&format!("limit=10&offset={offset}")));
        }

        let remote_models = request_json_file(url.as_str()).await?;
        let mut remote_models = serde_json::from_value::<Vec<ModelRaw>>(remote_models)?;
        let break_ = remote_models.len() < 10;
        result.append(&mut remote_models);

        if break_ {
            break;
        }

        offset += 10;
    }

    Ok(result)
}

// If there's nothing to update, it returns `None`. Otherwise, it returns the updated model.
fn update_model(
    old: &Model,
    new: &Model,
) -> Option<Model> {
    let mut result = old.clone();
    let mut has_update = false;

    if old.api_name != new.api_name {
        result.api_name = new.api_name.to_string();
        has_update = true;
    }

    if old.can_read_images != new.can_read_images {
        result.can_read_images = new.can_read_images;
        has_update = true;
    }

    if old.api_provider != new.api_provider {
        result.api_provider = new.api_provider.clone();
        has_update = true;
    }

    if old.dollars_per_1b_input_tokens != new.dollars_per_1b_input_tokens {
        result.dollars_per_1b_input_tokens = new.dollars_per_1b_input_tokens;
        has_update = true;
    }

    if old.dollars_per_1b_output_tokens != new.dollars_per_1b_output_tokens {
        result.dollars_per_1b_output_tokens = new.dollars_per_1b_output_tokens;
        has_update = true;
    }

    if new.explanation.is_some() && old.explanation != new.explanation {
        result.explanation = new.explanation.clone();
        has_update = true;
    }

    if new.api_env_var.is_some() && old.api_env_var != new.api_env_var {
        result.api_env_var = new.api_env_var.clone();
        has_update = true;
    }

    if has_update {
        Some(result)
    }

    else {
        None
    }
}
