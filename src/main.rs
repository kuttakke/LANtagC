use chrono::{NaiveDateTime, TimeZone, Utc};
use clap::Parser;
use owo_colors::OwoColorize;
use regex::Regex;
use scraper::{Html, Selector};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::fs::{read, write};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use strsim::normalized_damerau_levenshtein;
use tabled::{builder::Builder, settings::Style};
use tabled::{settings::object::Columns, settings::Format};
use tokio::task;
use url::form_urlencoded;

/// 为Lanraragi的作品增添中文标签，仅限无标签作品
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, styles=get_styles())]
struct Args {
    /// Lanraragi的URL;例 192.168.0.1:3000
    #[arg(short, long)]
    endpoint: String,

    /// Lanraragi的API key
    #[arg(short, long)]
    api_key: String,

    /// EX的Cookies;格式为：`igneous=xxx; ipb_member_id=xxx; ipb_pass_hash=xxx`
    #[arg(short, long)]
    cookies: String,
}

fn args() -> &'static Args {
    static ARGS: OnceLock<Args> = OnceLock::new();
    ARGS.get_or_init(|| {
        let args = Args::parse();
        args
    })
}

pub fn get_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .usage(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))),
        )
        .header(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))),
        )
        .literal(
            anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
        )
        .invalid(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))),
        )
        .error(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))),
        )
        .valid(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
        )
        .placeholder(
            anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::White))),
        )
}

#[derive(Debug)]
enum FetchError {
    Reqwest(reqwest::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    Other(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Reqwest(e) => write!(f, "Reqwest error: {}", e),
            FetchError::Io(e) => write!(f, "IO error: {}", e),
            FetchError::Json(e) => write!(f, "JSON error: {}", e),
            FetchError::Other(e) => write!(f, "Other error: {}", e),
        }
    }
}

impl Error for FetchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FetchError::Reqwest(e) => Some(e),
            FetchError::Io(e) => Some(e),
            FetchError::Json(e) => Some(e),
            FetchError::Other(_) => None,
        }
    }
}

impl From<serde_json::Error> for FetchError {
    fn from(err: serde_json::Error) -> Self {
        FetchError::Json(err)
    }
}

impl From<reqwest::Error> for FetchError {
    fn from(err: reqwest::Error) -> Self {
        FetchError::Reqwest(err)
    }
}

impl From<std::io::Error> for FetchError {
    fn from(err: std::io::Error) -> Self {
        FetchError::Io(err)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Archive {
    arcid: String,
    extension: String,
    isnew: String,
    lastreadtime: i64,
    pagecount: i32,
    progress: i32,
    tags: String,
    title: String,
}

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
struct GL {
    type_: String,
    datetime: String,
    tags: String,
    title: String,
    pages: String,
    url: String,
}

static NO_HANDLE_FILE_NAME: &str = "no_handle.json";

// fn endpoint() -> &'static String {
//     static ENDPOINT: OnceLock<String> = OnceLock::new();
//     ENDPOINT.get_or_init(|| {
//         let mut input = String::new();
//         println!("请输入LANraragi的URL(例如 192.168.0.1:3000)：");
//         std::io::stdin().read_line(&mut input).expect("读取失败");
//         input.trim().to_string()
//     })
// }

// fn apikey() -> &'static String {
//     static APIEKY: OnceLock<String> = OnceLock::new();
//     APIEKY.get_or_init(|| {
//         let mut input = String::new();
//         println!("请输入LANraragi的APIKEY(例如 123)：");
//         std::io::stdin().read_line(&mut input).expect("读取失败");
//         input.trim().to_string()
//     })
// }

// fn ex_cookies() -> &'static String {
//     static EX_COOKIES: OnceLock<String> = OnceLock::new();
//     EX_COOKIES.get_or_init(|| {
//         let mut input = String::new();
//         println!(
//             "请输入exhentai的COOKIES：\n格式为：igneous=xxx; ipb_member_id=xxx; ipb_pass_hash=xxx"
//         );
//         std::io::stdin().read_line(&mut input).expect("读取失败");
//         input.trim().to_string()
//     })
// }

fn no_handle_file() -> &'static Mutex<Vec<Archive>> {
    static NO_HANDLE_FILE: OnceLock<Mutex<Vec<Archive>>> = OnceLock::new();
    NO_HANDLE_FILE.get_or_init(|| {
        // check if file exists
        if !Path::new(NO_HANDLE_FILE_NAME).exists() {
            return Mutex::new(Vec::new());
        }
        let no_handle_data = String::from_utf8(read(NO_HANDLE_FILE_NAME).unwrap()).unwrap();
        if no_handle_data.is_empty() {
            return Mutex::new(Vec::new());
        }
        Mutex::new(serde_json::from_str::<Vec<Archive>>(&no_handle_data).unwrap())
    })
}

fn add_and_save_no_handle(data: Archive) {
    let mut no_handle_array = no_handle_file().lock().unwrap();
    no_handle_array.push(data);
    write(
        NO_HANDLE_FILE_NAME,
        serde_json::to_string_pretty(&*no_handle_array)
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
}

async fn fetch_lateast_cn_tag() -> Result<serde_json::Value, FetchError> {
    let mut retry_count = 0;
    let url = "https://github.com/EhTagTranslation/Database/releases/latest/download/db.text.json";
    loop {
        match reqwest::get(url).await {
            Ok(response) if response.status().is_success() => {
                // return Ok(response.json::<T>().await?);
                return Ok(response.json().await?);
            }
            Ok(_) => {
                retry_count += 1;
                if retry_count >= 3 {
                    return Err(FetchError::Other("Max retries reached".to_string()));
                }
            }
            Err(e) => {
                retry_count += 1;
                if retry_count >= 3 {
                    return Err(FetchError::Reqwest(e));
                }
            }
        }
    }
}

fn parse_data(
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

async fn fetch<T>(url: &str) -> Result<T, FetchError>
where
    T: DeserializeOwned,
{
    let mut retry_count = 0;

    loop {
        match reqwest::get(url).await {
            Ok(response) if response.status().is_success() => {
                return Ok(response.json::<T>().await?);
            }
            Ok(_) => {
                retry_count += 1;
                if retry_count >= 3 {
                    return Err(FetchError::Other("Max retries reached".to_string()));
                }
            }
            Err(e) => {
                retry_count += 1;
                if retry_count >= 3 {
                    return Err(FetchError::Reqwest(e));
                }
            }
        }
    }
}

fn is_empty_tags(tags: &str) -> bool {
    let tag_list: Vec<&str> = tags.split(",").collect();
    if tag_list.is_empty() {
        return true;
    }
    if tag_list.len() == 1 && tag_list[0].starts_with("date_added") {
        return true;
    } else {
        return false;
    }
}

fn get_regex_title(title: &str) -> String {
    if let Ok(regex) = Regex::new(
        r"(\[.*?\])\s*(.*?)\s*(?:#.*?)?\s*(?:\([^)]*\))?\s*(?:｜|︱)?\s*(?:\([^)]*\))?\s*(\[|$)",
    ) {
        if let Some(captures) = regex.captures(title) {
            if let Some(group) = captures.get(2) {
                println!("match group: {}", group.as_str());
                return group.as_str().to_string();
            }
        }
    }
    String::new()
}

async fn fetch_eh_with_retry(url: &str) -> Result<reqwest::Response, FetchError> {
    let mut retry_count = 0;

    loop {
        match reqwest::Client::new()
        .get(url)
        .header("Cookie", &args().cookies)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36 Edg/119.0.0.0")
        .send()
        .await {
            Ok(response) if response.status().is_success() => {
                return Ok(response);
            }
            Ok(_) => {
                retry_count += 1;
                if retry_count >= 3 {
                    return Err(FetchError::Other("Max retries reached".to_string()));
                }
            }
            Err(e) => {
                retry_count += 1;
                if retry_count >= 3 {
                    return Err(FetchError::Reqwest(e));
                }
            }
        }
    }
}

async fn search_from_eh(title: &str) -> Vec<GL> {
    let url = "https://exhentai.org/?f_search=";
    let url = format!(
        "{}{}",
        url,
        form_urlencoded::byte_serialize(title.as_bytes()).collect::<String>()
    );

    let resp = fetch_eh_with_retry(&url).await.unwrap();

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
            .select(&Selector::parse("td:nth-child(2) div:nth-child(3) div:nth-child(1)").unwrap())
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
            type_: type_,
            datetime: datetime,
            tags: tags,
            title: title,
            pages: pages,
            url: url,
        });
    }
    return gls;
}

fn print_and_get_index(gls: &Vec<GL>, archive: &Archive) -> i32 {
    let mut builder = Builder::default();
    builder.push_record(["序号", "标题", "相似度", "页数", "日期"]);
    for (i, gl) in gls.iter().enumerate() {
        let similarity = normalized_damerau_levenshtein(&archive.title, &gl.title) * 100.0;
        let page_flag = if &gl.pages.trim_end_matches(" pages") == &archive.pagecount.to_string() {
            format!("{}✅", &gl.pages)
        } else {
            (&gl.pages).to_string()
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
        &archive.title.bright_blue(),
        &archive.pagecount.bright_blue()
    );
    println!("{}", table);
    // get user input
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    let index = input.trim().parse::<i32>().unwrap();
    return index;
}

async fn change_tags_to_lanraragi(archive: &Archive, tags: &str) {
    let url = format!(
        "http://{}/api/archives/{}/metadata",
        &args().endpoint,
        &archive.arcid
    );
    // put method with form data
    let form_data = vec![
        ("tags", tags),
        ("title", &archive.title),
        ("key", &args().api_key),
    ];
    let resp = reqwest::Client::new()
        .put(url)
        .form(&form_data)
        .send()
        .await
        .unwrap();
    if resp.status().is_success() {
        println!("to -> {}", tags.bright_cyan());
    } else {
        // print the text
        for line in resp.text().await.unwrap().lines() {
            println!("{}", line.red());
        }
        // println!("failed");
        panic!("failed")
    }
}

async fn get_tags_from_eh_gl(gl: &GL, cn_tags: &HashMap<String, String>) -> String {
    let resp = fetch_eh_with_retry(&gl.url).await.unwrap();
    let text = resp.text().await.unwrap();
    let document = Html::parse_document(&text);
    let tag_bodu_selector = Selector::parse("div#taglist > table a").unwrap();
    let tag_body = document.select(&tag_bodu_selector);
    let mut tags_str = String::new();
    for i in tag_body {
        let tag = i.value().attr("id").unwrap();
        let mut raw_tag = String::new();
        raw_tag.push_str(&tag.trim_start_matches("ta_").replace("_", " "));
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
        &gl.url.trim_start_matches("https://")
    ));
    let datetime = NaiveDateTime::parse_from_str(&gl.datetime, "%Y-%m-%d %H:%M")
        .expect("failed to parse datetime");
    let utc_time = Utc.from_utc_datetime(&datetime).timestamp();
    tags_str.push_str(&format!(",timestamp:{}", utc_time));
    return tags_str;
}

// 异步函数，获取所有lanraragi作品
async fn fetch_archives() -> Vec<Archive> {
    fetch(&format!("http://{}/api/archives", &args().endpoint))
        .await
        .unwrap()
}

// 异步函数，获取最新的cn标签
async fn fetch_latest_cn_tag() -> serde_json::Value {
    fetch_lateast_cn_tag().await.unwrap()
}

#[tokio::main]
async fn main() {
    // endpoint();
    // apikey();
    // ex_cookies();
    args();
    no_handle_file();
    // println!("获取所有lanraragi作品...");
    // let all_archive: Vec<Archive> = fetch(&format!("http://{}/api/archives", &args().endpoint))
    //     .await
    //     .unwrap();
    // println!("获取最新的cn标签...");
    // let tags = fetch_lateast_cn_tag().await.unwrap();
    println!("获取Lanraragi作品和最新的cn标签...");
    let fetch_task = task::spawn(fetch_archives());
    let tags_task = task::spawn(fetch_latest_cn_tag());

    // 等待两个任务完成
    let all_archive = fetch_task.await.unwrap();
    let tags = tags_task.await.unwrap();

    let tag_cn = parse_data(&tags).unwrap();
    let mut run_count = 0;

    // let mut no_handle_array: Vec<&Archive> = Vec::new();
    println!("共有 {} 条作品", all_archive.len().bright_green());
    for archive in all_archive.iter() {
        if !is_empty_tags(&archive.tags) {
            run_count += 1;
            continue;
        }
        let title = get_regex_title(&archive.title);
        if title == "".to_string() {
            println!("❌title no match: {}", &archive.title.red());
            // no_handle_array.push(archive);
            add_and_save_no_handle(archive.clone());
            run_count += 1;
            continue;
        }
        let gls = search_from_eh(&title).await;
        if gls.is_empty() {
            println!("❌no result for: {}", &archive.title.red());
            // no_handle_array.push(archive);
            add_and_save_no_handle(archive.clone());
            run_count += 1;
            continue;
        }
        let index = print_and_get_index(&gls, &archive);
        if index > 0 && index <= gls.len() as i32 {
            let tags = get_tags_from_eh_gl(&gls[index as usize - 1], &tag_cn).await;
            change_tags_to_lanraragi(&archive, &format!("{},{}", &archive.tags, &tags)).await;
            run_count += 1;
            println!(
                "已处理 {}/{}  {:.1}%",
                run_count.green(),
                all_archive.len().cyan(),
                (run_count as f32 / all_archive.len() as f32 * 100.0).bright_green()
            );
        } else {
            // no_handle_array.push(archive);
            add_and_save_no_handle(archive.clone());
            run_count += 1;
            print!("❌not handle");
        }
    }
    println!("结束");
}
