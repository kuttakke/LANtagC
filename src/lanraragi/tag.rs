use std::collections::HashMap;

use super::error::FetchError;
use super::progress::make_progress_bar;
use super::utils::fetch_raw_with_retry;

async fn fetch() -> Result<serde_json::Value, FetchError> {
    let resp = fetch_raw_with_retry(|| {
        reqwest::Client::new().get(
            "https://github.com/EhTagTranslation/Database/releases/latest/download/db.text.json",
        )
    })
    .await?;
    Ok(serde_json::from_slice::<serde_json::Value>(
        &make_progress_bar(resp, "cn tag").await?,
    )?)
}

pub fn parse_data(
    data: &serde_json::Value,
) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let data_array = data["data"].as_array().ok_or("invalid data")?;

    let mut tags: HashMap<String, String> = HashMap::new();

    for item in data_array {
        let namespace = item["namespace"].as_str().unwrap_or_default();
        let front_matters = item["frontMatters"]["name"].as_str().unwrap_or_default();

        let namespace = match namespace {
            "reclass" => "category",
            _ => namespace,
        };

        let mut tag_name: &str;
        let mut tag_cn_name: &str;

        if let Some(name) = item["data"].as_object() {
            for (key, value) in name {
                tag_name = match key.as_str() {
                    "artistacg" => "artist cg",
                    "gamecg" => "game cg",
                    "imageset" => "image set",
                    _ => key.as_str(),
                };

                if let Some(tag_data) = value.as_object() {
                    tag_cn_name = tag_data["name"].as_str().unwrap_or_default();
                    let tag = format!("{}:{}", front_matters, tag_cn_name);
                    tags.insert(format!("{}:{}", namespace, tag_name), tag);
                }
            }
        }
    }

    Ok(tags)
}

// 异步函数，获取最新的cn标签
pub async fn fetch_latest_cn_tag() -> serde_json::Value {
    match fetch().await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("fetch latest cn tag failed: {}", e);
            std::process::exit(1);
        }
    }
}
