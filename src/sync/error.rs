use chrono::prelude::*;

#[derive(Debug)]
pub enum Error {
    /// API rate limit reached, try again at
    ApiRateLimit(DateTime<Utc>),
    /// Requested key doesn't exist
    NoSuchKey(String),
    /// HTTP connection error
    Http(reqwest::Error),
    /// Generic IO error
    Io(std::io::Error),
    /// Invalid credentials
    InvalidCredentials(String),
    /// Miscancellous API errors
    Misc(String),
    /// Invalid or missing configuration item
    ConfigurationItem,
    /// Remote has not been set
    NoRemoteSet,
    /// Remote is not a proper vpass remote
    InvalidRemote,
    /// Key already exists, not overwriting
    KeyAlreadyExists(String),
}
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::Io(error)
    }
}
impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Error::Http(error)
    }
}
