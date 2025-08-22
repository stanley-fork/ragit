use crate::error::Error;
use chrono::{DateTime, Local};
use lazy_static::lazy_static;
use ragit_fs::write_log;
use serde::de::DeserializeOwned;
use sha3::{Digest, Sha3_256};
use std::collections::HashMap;
use std::sync::RwLock;

pub fn trim_long_string(s: &str, prefix_len: usize, suffix_len: usize) -> String {
    if s.len() <= (prefix_len + suffix_len) || s.chars().count() <= (prefix_len + suffix_len) {
        s.to_string()
    }

    else {
        format!(
            "{}...{}",
            s.chars().take(prefix_len).collect::<String>(),
            s.chars().rev().take(suffix_len).collect::<String>().chars().rev().collect::<String>(),
        )
    }
}

pub fn into_query_string(form: &HashMap<String, String>) -> String {
    form.iter().map(|(key, value)| format!("{key}={}", url_encode_strict(value))).collect::<Vec<_>>().join("&")
}

pub fn format_duration(seconds: i64) -> String {
    let suffix = if seconds < 0 {
        if seconds.abs() < 2 {
            return String::from("just now");
        }

        "later"
    } else {
        "ago"
    };

    let s = seconds.abs() as u64;

    let prefix = if s < 99 {
        format!("{} seconds", s)
    }

    else if s < 60 * 99 {
        format!("{} minutes", s / 60)
    }

    else if s < 60 * 60 * 99 {
        format!("{} hours", s / 60 / 60)
    }

    else {
        format!("{} days", s / 60 / 60 / 24)
    };

    format!("{prefix} {suffix}")
}

pub fn render_time(value: &tera::Value, args: &HashMap<String, tera::Value>) -> Result<tera::Value, tera::Error> {
    match value {
        tera::Value::Number(n) if n.is_i64() => {
            let now = Local::now().timestamp();
            Ok(tera::Value::String(format_duration(now - n.as_i64().unwrap())))
        },
        tera::Value::String(s) => match DateTime::parse_from_rfc3339(s) {
            Ok(t) => {
                let now = Local::now().timestamp();
                Ok(tera::Value::String(format_duration(now - t.timestamp())))
            },
            _ => {
                write_log(
                    "render_time",
                    &format!("render_time({value:?}, {args:?}) expects an timestamp, but got {value:?}"),
                );
                Ok(tera::Value::String(String::from("err")))
            },
        },
        _ => {
            write_log(
                "render_time",
                &format!("render_time({value:?}, {args:?}) expects an timestamp, but got {value:?}"),
            );
            Ok(tera::Value::String(String::from("err")))
        },
    }
}

pub fn int_comma(value: &tera::Value, args: &HashMap<String, tera::Value>) -> Result<tera::Value, tera::Error> {
    let n = match value {
        tera::Value::Number(n) if n.is_i64() => n.as_i64().unwrap(),
        tera::Value::String(s) => match s.parse::<i64>() {
            Ok(n) => n,
            _ => {
                write_log(
                    "render_time",
                    &format!("int_comma({value:?}, {args:?}) expects an integer, but got {value:?}"),
                );
                return Ok(tera::Value::String(String::from("err")));
            },
        },
        _ => {
            write_log(
                "render_time",
                &format!("int_comma({value:?}, {args:?}) expects an integer, but got {value:?}"),
            );
            return Ok(tera::Value::String(String::from("err")));
        },
    };
    let is_neg = n < 0;
    let n = n.abs() as u64;
    let s = match n {
        ..=999 => n.to_string(),
        ..=999_999 =>             format!("{},{:03}",                                                                                          n / 1000,        n % 1000),
        ..=999_999_999 =>         format!("{},{:03},{:03}",                                                              n / 1_000_000,        n / 1000 % 1000, n % 1000),
        ..=999_999_999_999 =>     format!("{},{:03},{:03},{:03}",                              n / 1_000_000_000,        n / 1_000_000 % 1000, n / 1000 % 1000, n % 1000),
        ..=999_999_999_999_999 => format!("{},{:03},{:03},{:03},{:03}", n / 1_000_000_000_000, n / 1_000_000_000 % 1000, n / 1_000_000 % 1000, n / 1000 % 1000, n % 1000),
        _ => n.to_string(),  // who cares
    };
    let s = format!("{}{s}", if is_neg { "-" } else { "" });

    Ok(tera::Value::String(s))
}

pub fn hash_str(s: &str) -> String {
    let mut hasher = Sha3_256::new();
    hasher.update(s.as_bytes());
    format!("{:064x}", hasher.finalize())
}

pub fn uri_from_str(s: &str) -> warp::http::Uri {
    match warp::http::Uri::try_from(s) {
        Ok(u) => u,
        Err(e) => {
            write_log(
                "uri_from_str",
                &format!("uri_from_str({:?}) failed with {e:?}", trim_long_string(&s, 80, 80)),
            );

            // TODO: it shall not panic
            panic!()
        },
    }
}

pub fn url_encode_strict(s: &str) -> String {
    let mut result = vec![];

    for b in s.as_bytes() {
        if is_safe_char(b) {
            result.push(*b);
        }

        else {
            result.push(b'%');

            for c in format!("{b:02X}").as_bytes() {
                result.push(*c);
            }
        }
    }

    String::from_utf8(result).unwrap()
}

fn is_safe_char(b: &u8) -> bool {
    match *b {
        b'0'..=b'9'
        | b'a'..=b'z'
        | b'A'..=b'Z'
        | b'-' | b'_' => true,
        _ => false,
    }
}

// Bots are DDos-ing ragithub. I have to protect ragithub from them.
lazy_static! {
    static ref FETCH_JSON_CACHE: RwLock<HashMap<String, serde_json::Value>> = RwLock::new(HashMap::new());
}

static mut FETCH_JSON_CACHE_CLEAR_AT: i64 = 0;

pub async fn fetch_json<T: DeserializeOwned>(url: &str, api_key: &Option<String>) -> Result<T, Error> {
    let dt = unsafe { Local::now().timestamp() - FETCH_JSON_CACHE_CLEAR_AT };
    let mut value: Option<serde_json::Value> = if let Ok(cache) = FETCH_JSON_CACHE.try_read() {
        if dt > 300 {
            // outdated data
            None
        }

        else if let Some(v) = cache.get(url) {
            write_log(
                "fetch_json: cache-hit",
                &format!("fetch_json({url:?})"),
            );
            Some(v.clone())
        }

        else {
            None
        }
    }

    else {
        None
    };

    if value.is_none() {
        let mut request = reqwest::Client::new().get(url);

        if let Some(api_key) = api_key {
            request = request.header("x-api-key", api_key);
        }

        let response = request.send().await?;
        write_log(
            "fetch_json: cache-miss",
            &format!("fetch_json({url:?}) got `{}`", response.status()),
        );

        let v: serde_json::Value = response.json().await?;

        // For now, we don't need a background worker that clears the cache.
        // There's a background worker that fetches something from the server
        // every 5 minutes (same cycle). The worker will call this function,
        // so we already kinda have a background worker.
        unsafe {
            if let Ok(mut cache) = FETCH_JSON_CACHE.try_write() {
                if cache.len() > 512 {
                    cache.clear();
                    FETCH_JSON_CACHE_CLEAR_AT = Local::now().timestamp();
                    write_log(
                        "fetch_json: cache-clear",
                        &format!("cleared cache because `cache.len() > 512`"),
                    );
                }

                else if dt > 300 {
                    cache.clear();
                    FETCH_JSON_CACHE_CLEAR_AT = Local::now().timestamp();
                    write_log(
                        "fetch_json: cache-clear",
                        &format!("cleared cache because more than 5 mins have passed since the last cache-clear"),
                    );
                }

                cache.insert(url.to_string(), v.clone());
            }
        }

        value = Some(v);
    }

    Ok(serde_json::from_value(value.unwrap())?)
}

pub async fn fetch_text(url: &str, api_key: &Option<String>) -> Result<String, Error> {
    let mut request = reqwest::Client::new().get(url);

    if let Some(api_key) = api_key {
        request = request.header("x-api-key", api_key);
    }

    let response = request.send().await?;
    write_log(
        "fetch_text",
        &format!("fetch_text({url:?}) got `{}`", response.status()),
    );

    Ok(response.text().await?)
}

pub async fn fetch_bytes(url: &str, api_key: &Option<String>) -> Result<Vec<u8>, Error> {
    let mut request = reqwest::Client::new().get(url);

    if let Some(api_key) = api_key {
        request = request.header("x-api-key", api_key);
    }

    let response = request.send().await?;
    write_log(
        "fetch_bytes",
        &format!("fetch_bytes({url:?}) got `{}`", response.status()),
    );

    Ok(response.bytes().await?.to_vec())
}
