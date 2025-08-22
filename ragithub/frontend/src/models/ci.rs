use chrono::{DateTime, Utc};
use crate::error::Error;
use crate::utils::hash_str;
use ragit_fs::{
    join,
    read_string,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiIndex {
    pub title: String,
    pub ended_at: DateTime<Utc>,
    pub git_title: String,
    pub ragit_version: String,
    pub pass: usize,
    pub fail: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiMeta {
    pub complete: bool,
    pub started_at: DateTime<Utc>,
    pub commit: String,
    pub platform: CiPlatform,
    pub rand_seed: u64,
    pub ended_at: DateTime<Utc>,
    pub elapsed_ms: usize,
    pub ragit_version: String,
    pub commit_title: String,
    pub commit_message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiHistoryMeta {
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiPlatform {
    pub cargo_version: String,
    pub rustc_version: String,
    pub python_version: String,
    pub platform: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiDetail {
    pub meta: CiMeta,
    pub tests: HashMap<String, CiCaseResult>,
    pub result: CiTestResult,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiHistoryDetail {
    pub meta: CiHistoryMeta,
    pub tests: HashMap<String, CiCaseResult>,
    pub result: CiTestResult,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiCaseResult {
    pub seq: usize,
    pub pass: bool,
    pub error: Option<String>,
    pub message: Option<String>,
    pub elapsed_ms: usize,

    // We don't need this for test results,
    // but we do need this for case histories.
    pub ended_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiTestResult {
    pub total: usize,
    pub complete: usize,
    pub pass: usize,
    pub fail: usize,
    pub remaining: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RenderableCiCaseResult {
    pub name: String,
    pub name_hash: String,
    pub seq: usize,
    pub pass: bool,
    pub error: Option<String>,
    pub message: Option<String>,
    pub elapsed_ms: usize,
    pub ended_at: Option<DateTime<Utc>>,
    pub history_mode: bool,
}

pub fn into_renderables(
    results: &HashMap<String, CiCaseResult>,
    history_mode: bool,
) -> Result<Vec<RenderableCiCaseResult>, Error> {
    let mut cases = results.iter().map(
        |(name, t)| RenderableCiCaseResult {
            name: name.to_string(),
            name_hash: hash_str(name).get(0..9).unwrap().to_string(),
            seq: t.seq,
            pass: t.pass,
            error: t.error.clone(),
            message: t.message.clone(),
            elapsed_ms: t.elapsed_ms,
            ended_at: t.ended_at,
            history_mode,
        }
    ).collect::<Vec<_>>();
    cases.sort_by_key(|t| t.seq);

    Ok(cases)
}

// `_index.json` is sorted in descending order. So 1 is prev and -1 is next.
pub fn get_prev_detail(title: &str) -> Result<Option<String>, Error> {
    get_detail_by_title_and_offset(title, 1)
}

pub fn get_next_detail(title: &str) -> Result<Option<String>, Error> {
    get_detail_by_title_and_offset(title, -1)
}

fn get_detail_by_title_and_offset(title: &str, offset: i32) -> Result<Option<String>, Error> {
    let j = read_string(&join("test-results", "_index.json")?)?;
    let index = serde_json::from_str::<Vec<CiIndex>>(&j)?;
    let target_at = index.iter().position(|d| d.title == title);

    if let Some(target_at) = target_at {
        let position = (target_at as i32 + offset + index.len() as i32) as usize % index.len();

        if let Some(CiIndex { title, .. }) = index.get(position) {
            Ok(Some(title.to_string()))
        }

        else {
            Ok(None)
        }
    }

    else {
        Ok(None)
    }
}
