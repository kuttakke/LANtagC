use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use regex::Regex;

use super::utils::fetch;

use super::args::args;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Archive {
    pub arcid: String,
    pub extension: String,
    pub isnew: String,
    pub lastreadtime: i64,
    pub pagecount: i32,
    pub progress: i32,
    pub tags: String,
    pub title: String,
}

impl Archive {
    pub async fn change_tags_to_lanraragi(&self, tags: &str) {
        let url = format!(
            "http://{}/api/archives/{}/metadata",
            &args().endpoint,
            &self.arcid
        );
        // put method with form data
        let form_data = vec![
            ("tags", tags),
            ("title", &self.title),
            ("key", &args().api_key),
        ];
        // no need to re-try
        let resp = reqwest::Client::new()
            .put(url)
            .form(&form_data)
            .send()
            .await
            .unwrap();
        if resp.status().is_success() {
            println!("to -> {}", tags.bright_cyan());
        } else {
            for line in resp.text().await.unwrap().lines() {
                println!("{}", line.red());
            }
            panic!("failed")
        }
    }
    
    pub fn is_empty_tags(&self) -> bool {
        let tag_list: Vec<&str> = self.tags.split(',').collect();
        if tag_list.is_empty() {
            return true
        }
        tag_list.len() == 1 && tag_list[0].starts_with("date_added")
    }
    
    pub fn regex_title(&self) -> String {
        if let Ok(regex) = Regex::new(
            r"(\[.*?\])\s*(.*?)\s*(?:#.*?)?\s*(?:\([^)]*\))?\s*(?:｜|︱.*?)?\s*(?:\([^)]*\))?\s*(\[|$)",
        ) {
            if let Some(captures) = regex.captures(&self.title) {
                if let Some(group) = captures.get(2) {
                    println!("match group: {}", group.as_str());
                    return group.as_str().to_string();
                }
            }
        }
        String::new()
    }
    
    // 异步函数，获取所有lanraragi作品
    pub async fn fetch_archives() -> Vec<Self> {
        fetch(&format!("http://{}/api/archives", &args().endpoint))
            .await
            .unwrap()
    }

}

