use super::error::FetchError;

use clap::error::Result;
use serde::de::DeserializeOwned;

pub async fn fetch_raw_with_retry<F>(builder: F) -> Result<reqwest::Response, FetchError>
where
    F: Fn() -> reqwest::RequestBuilder,
{
    let mut retry_count = 0;

    loop {
        match builder().send().await {
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

pub async fn fetch<T>(url: &str) -> Result<T, FetchError>
where
    T: DeserializeOwned,
{
    Ok(fetch_raw_with_retry(|| reqwest::Client::new().get(url)).await?.json::<T>().await?)
}