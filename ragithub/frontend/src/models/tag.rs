use async_std::task;
use crate::error::Error;
use crate::methods::get_backend;
use crate::models::AiModel;
use crate::utils::fetch_json;
use ragit_fs::{
    WriteMode,
    read_string,
    write_string,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tag {
    pub name: String,
    pub selected: bool,
    pub href: String,
}

// There are multiple readers and single writer. The writer writes to the
// file very rarely, so it's easy to avoid data-race: reading it again
// after a moment will fix!
pub async fn load_tags() -> Result<Vec<String>, Error> {
    for i in 0..3 {
        match load_tags_worker() {
            Ok(r) => { return Ok(r); },
            Err(e) => {
                if i == 2 {
                    return Err(e);
                } else {
                    task::sleep(Duration::from_millis(200)).await;
                }
            },
        }
    }

    unreachable!()
}

fn load_tags_worker() -> Result<Vec<String>, Error> {
    let t = read_string("tags.json")?;
    Ok(serde_json::from_str(&t)?)
}

fn add_tags(tags: &[String]) -> Result<(), Error> {
    let t = read_string("tags.json")?;
    let new_tags = vec![
        tags.to_vec(),
        serde_json::from_str::<Vec<String>>(&t)?,
    ].concat()
        .into_iter()
        .collect::<HashSet<String>>()  // dedup
        .into_iter()
        .collect::<Vec<String>>();

    Ok(write_string(
        "tags.json",
        &serde_json::to_string_pretty(&new_tags)?,
        WriteMode::Atomic,
    )?)
}

// Someday, we might have a lot of ai models and it'd be inefficient to fetch all the models.
const AI_MODEL_SEARCH_LIMIT: usize = 2000;

pub async fn fetch_tags() -> Result<(), Error> {
    let backend = get_backend();
    let ai_models = fetch_json::<Vec<AiModel>>(&format!("{backend}/ai-model-list?limit={AI_MODEL_SEARCH_LIMIT}"), &None).await?;
    let reached_limit = ai_models.len() == AI_MODEL_SEARCH_LIMIT;
    let mut tags = ai_models.into_iter().map(
        |ai_model| ai_model.tags
    ).filter(
        |tag| !tag.is_empty()
    ).collect::<Vec<Vec<String>>>().concat();

    // In this case, we cannot get the full list of tags. All we can do is to
    // add the tag lists to existing `tags.json`.
    if reached_limit {
        add_tags(&tags)?;
    }

    else {
        tags.sort();
        tags.dedup();
        write_string(
            "tags.json",
            &serde_json::to_string_pretty(&tags)?,
            WriteMode::Atomic,
        )?;
    }

    Ok(())
}
