use super::error::FetchError;

use futures_util::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use reqwest::Response;
use std::cmp::min;
use std::sync::OnceLock;

pub fn multi_progress() -> &'static MultiProgress {
    static PROGRESS: OnceLock<MultiProgress> = OnceLock::new();
    PROGRESS.get_or_init(MultiProgress::new)
}

pub async fn make_progress_bar(resp: Response, name: &str) -> Result<Vec<u8>, FetchError> {
    let total = match resp.content_length() {
        Some(total) => total,
        None => return Err(FetchError::Other("No content length".to_string())),
    };

    // 计算vector应该给多少长度
    let mut buf = Vec::with_capacity(total as usize);
    let mut progress = 0u64;

    let bar = multi_progress().add(ProgressBar::new(total));
    bar.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        ?.progress_chars("#>-"));
    bar.set_message(format!("Downloading {}", name));

    // download chunk
    let mut stream = resp.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.or(Err(FetchError::Other("No chunk".to_string())))?;
        buf.extend_from_slice(&chunk);
        let position = min(progress + chunk.len() as u64, total);
        progress = position;
        bar.set_position(position);
    }
    bar.finish_with_message(format!("Download {} ✅", name).bright_red().to_string());
    Ok(buf)
}
