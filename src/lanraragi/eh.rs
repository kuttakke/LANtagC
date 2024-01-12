use super::args::args;
use super::error::FetchError;
use super::archive::Archive;
use super::utils::fetch_raw_with_retry;

use chrono::{NaiveDateTime, TimeZone, Utc};
use owo_colors::OwoColorize;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug};
use strsim::normalized_damerau_levenshtein;
use tabled::{builder::Builder, settings::Style};
use tabled::{settings::object::Columns, settings::Format};
use url::form_urlencoded;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct HenTagItem {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct HenTag {
    id: String,
    title: String,
    circles: Vec<HenTagItem>,
    artists: Vec<HenTagItem>,
    characters: Vec<HenTagItem>,
    male_tags: Vec<HenTagItem>,
    female_tags: Vec<HenTagItem>,
    other_tags: Vec<HenTagItem>,
    language: i32,
    category: i32,
    locations: Vec<String>,
    created_at: i32,
    last_modified: i32,
    cover_image_url: String,
    favorite: bool,
    is_controversial: bool,
    is_dead: bool,
    is_pending_approval: bool,
    #[serde(default = "default_tags")]
    tags: String,
}

fn default_tags() -> String {
    "".to_string()
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GL {
    type_: String,
    datetime: String,
    tags: String,
    title: String,
    pages: String,
    url: String,
}

async fn fetch_eh(url: &str) -> Result<reqwest::Response, FetchError> {
    fetch_raw_with_retry(
        || reqwest::Client::new()
        .get(url)
        .header("Cookie", &args().cookies)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36 Edg/119.0.0.0")
    ).await
}

impl Archive {
    pub async fn search_from_eh(&self) -> Vec<GL> {
        let title = self.regex_title();
        if title.is_empty() {
            println!("❌title no match: {}", &self.title.red());
            return vec![];
        }

        let url = "https://exhentai.org/?f_search=";
        let url = format!(
            "{}{}",
            url,
            form_urlencoded::byte_serialize(title.as_bytes()).collect::<String>()
        );

        let resp = fetch_eh(&url).await.unwrap();

        let text = resp.text().await.unwrap();
        let doucment = Html::parse_document(&text);
        let trs_selector = Selector::parse("table.itg.gltc tr").unwrap();
        let trs = doucment.select(&trs_selector);

        let mut frist_flag = true;

        let mut gls = vec![];

        for tr in trs {
            if frist_flag {
                frist_flag = false;
                continue;
            }
            let type_ = tr
                .select(&Selector::parse("td:nth-child(1) div").unwrap())
                .next()
                .unwrap()
                .text()
                .collect::<String>();
            let datetime = tr
                .select(
                    &Selector::parse("td:nth-child(2) div:nth-child(3) div:nth-child(1)").unwrap(),
                )
                .next()
                .unwrap()
                .text()
                .collect::<String>();
            let url = tr
                .select(&Selector::parse("td:nth-child(3) a").unwrap())
                .next()
                .unwrap()
                .value()
                .attr("href")
                .unwrap()
                .to_string();
            let title = tr
                .select(&Selector::parse("td:nth-child(3) a div:nth-child(1)").unwrap())
                .next()
                .unwrap()
                .text()
                .collect::<String>();
            let pages = tr
                .select(&Selector::parse("td:nth-child(4) div:nth-child(2)").unwrap())
                .next()
                .unwrap()
                .text()
                .collect::<String>();
            let tagnods_selector =
                Selector::parse("td:nth-child(3) a div:nth-child(2) div.gt").unwrap();
            let tagnods = tr.select(&tagnods_selector);
            let mut tags = String::new();
            for tagnode in tagnods {
                if let Some(title) = tagnode.value().attr("title") {
                    tags.push_str(title);
                    tags.push(',');
                }
            }

            // Remove trailing comma
            tags.pop();

            gls.push(GL {
                type_,
                datetime,
                tags,
                title,
                pages,
                url,
            });
        }
        if gls.is_empty() {
            println!("❌no result for: {}", &self.title.red());
        }
        gls
    }

    pub fn print_and_get_index(&self, gls: &[GL]) -> i32 {
        let mut builder = Builder::default();
        builder.push_record(["序号", "标题", "相似度", "页数", "日期"]);
        for (i, gl) in gls.iter().enumerate() {
            let similarity = normalized_damerau_levenshtein(&self.title, &gl.title) * 100.0;
            let page_flag =
                if gl.pages.trim_end_matches(" pages") == self.pagecount.to_string() {
                    format!("{}✅", &gl.pages)
                } else {
                    (gl.pages).to_string()
                };
            builder.push_record([
                &(i as i32 + 1).to_string(),
                &gl.title,
                &format!("{:.1}%", similarity).to_string(),
                &page_flag,
                &gl.datetime,
            ]);
        }
        let mut table = builder.build();
        table
            .with(Style::rounded())
            .modify(
                Columns::single(0),
                Format::content(|s| s.cyan().to_string()),
            )
            .modify(
                Columns::single(1),
                Format::content(|s| s.magenta().to_string()),
            )
            .modify(
                Columns::single(2),
                Format::content(|s| s.green().to_string()),
            )
            .modify(
                Columns::single(3),
                Format::content(|s| s.blue().to_string()),
            )
            .modify(
                Columns::single(4),
                Format::content(|s| s.yellow().to_string()),
            );
        println!(
            "{} - {}",
            &self.title.bright_blue(),
            &self.pagecount.bright_blue()
        );
        println!("{}", table);
        // get user input
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        input.trim().parse::<i32>().unwrap()
    }


}

impl GL {
    pub async fn get_tags_from_eh_gl(&self, cn_tags: &HashMap<String, String>) -> String {
        let resp = fetch_eh(&self.url).await.unwrap();
        let text = resp.text().await.unwrap();
        let document = Html::parse_document(&text);
        let tag_bodu_selector = Selector::parse("div#taglist > table a").unwrap();
        let tag_body = document.select(&tag_bodu_selector);
        let mut tags_str = String::new();
        for i in tag_body {
            let tag = i.value().attr("id").unwrap();
            let mut raw_tag = String::new();
            raw_tag.push_str(&tag.trim_start_matches("ta_").replace('_', " "));
            if let Some(cn_tag) = cn_tags.get(&raw_tag) {
                tags_str.push_str(cn_tag);
                tags_str.push(',');
            } else {
                tags_str.push_str(&raw_tag);
                tags_str.push(',');
            }
        }
        tags_str.push_str(&format!(
            "source:{}",
            &self.url.trim_start_matches("https://")
        ));
        let datetime = NaiveDateTime::parse_from_str(&self.datetime, "%Y-%m-%d %H:%M")
            .expect("failed to parse datetime");
        let utc_time = Utc.from_utc_datetime(&datetime).timestamp();
        tags_str.push_str(&format!(",timestamp:{}", utc_time));
        tags_str
    }
}