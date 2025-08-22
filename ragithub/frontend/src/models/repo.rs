use async_std::task;
use chrono::{DateTime, Utc};
use crate::error::Error;
use crate::methods::get_backend;
use crate::utils::fetch_json;
use ragit_fs::{
    WriteMode,
    read_string,
    write_log,
    write_string,
};
pub use ragit_server::models::repo::{Repository, Traffic};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

// FE uses `Vec<RepoIndex>` as a simple cache.
#[derive(Clone, Deserialize, Serialize)]
pub struct RepoIndex {
    pub id: i32,
    pub name: String,
    pub normalized_name: String,
    pub description: Option<String>,
    pub clone_total: usize,
    pub clone_weekly: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// There are multiple readers and single writer. The writer writes to the
// file very rarely, so it's easy to avoid data-race: reading it again
// after a moment will fix!
pub async fn load_repositories() -> Result<Vec<RepoIndex>, Error> {
    for i in 0..3 {
        match load_repositories_worker() {
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

fn load_repositories_worker() -> Result<Vec<RepoIndex>, Error> {
    let j = read_string("repositories.json")?;
    let j: Vec<RepoIndex> = serde_json::from_str(&j)?;
    Ok(j)
}

pub async fn fetch_repositories() -> Result<(), Error> {
    let backend = get_backend();
    let repositories = fetch_json::<Vec<Repository>>(&format!("{backend}/repo-list/sample"), &None).await?;
    let mut result = Vec::with_capacity(repositories.len());

    for repo in repositories.iter() {
        let mut traffic = fetch_json::<HashMap<String, Traffic>>(&format!("{backend}/sample/{}/traffic", repo.name), &None).await?;
        let total = traffic.remove("all").unwrap_or_else(
            || {
                write_log(
                    "fetch_repositories",
                    &format!("`{backend}/sample/{}/traffic` is missing \"all\" field.", repo.name),
                );
                Traffic {
                    push: 0,
                    clone: 0,
                }
            }
        );
        let mut traffic = traffic.into_iter().collect::<Vec<_>>();
        traffic.sort_by_key(|(date, _)| date.clone());
        traffic = traffic[(traffic.len() - 7)..].to_vec();
        let clone_weekly = traffic.iter().map(|(_, traffic)| traffic.clone as usize).sum();

        result.push(RepoIndex {
            id: repo.id,
            name: repo.name.clone(),
            normalized_name: repo.name.to_ascii_lowercase(),
            description: repo.description.clone(),
            created_at: repo.created_at,
            updated_at: repo.updated_at,
            clone_total: total.clone as usize,
            clone_weekly,
        });
    }

    result.sort_by_key(|r| r.name.clone());
    write_string(
        "repositories.json",
        &serde_json::to_string(&result)?,
        WriteMode::CreateOrTruncate,
    )?;

    Ok(())
}
