use chrono::prelude::*;

use serde_json::Value;

#[derive(Debug)]
pub enum Error {
    /// API rate limit reached, try again at
    ApiRateLimit(DateTime<Utc>),
    /// Requested key doesn't exist
    NoSuchKey(String),
    /// HTTP connection error
    Http(reqwest::Error),
    /// HTTP status code implies error.
    /// These should usually be handled before user sees them.
    HttpStatus(u16, Option<Value>),
    /// Generic IO error
    Io(std::io::Error),
    /// Invalid credentials
    InvalidCredentials(String),
    /// Miscancellous API error
    Misc(Value),
    /// Invalid or missing configuration item
    ConfigurationItem,
    /// Invalid update key was given
    InvalidUpdateKey,
    /// Remote has not been set
    NoRemoteSet,
    /// Remote is not a proper vpass remote
    InvalidRemote,
    /// Remote has insufficient security, e.g. a public git repo
    InsufficientSecurity(String),
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
