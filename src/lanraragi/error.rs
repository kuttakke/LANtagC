use std::error::Error;
use std::fmt::Debug;

#[derive(Debug)]
pub enum FetchError {
    Reqwest(reqwest::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    Bar(indicatif::style::TemplateError),
    Other(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Reqwest(e) => write!(f, "Reqwest error: {}", e),
            FetchError::Io(e) => write!(f, "IO error: {}", e),
            FetchError::Json(e) => write!(f, "JSON error: {}", e),
            FetchError::Bar(e) => write!(f, "Bar error: {}", e),
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
            FetchError::Bar(e) => Some(e),
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

impl From<indicatif::style::TemplateError> for FetchError {
    fn from(err: indicatif::style::TemplateError) -> Self {
        FetchError::Bar(err)
    }
}
