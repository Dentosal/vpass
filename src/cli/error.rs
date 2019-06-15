use std::io;
use std::path::PathBuf;

use super::validate::ValidationError;

#[must_use]
pub type VResult<T> = Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// Generic IO error
    Io(io::Error),
    /// Config file missing or path not pointing to a file
    ConfigNotFound(PathBuf),
    /// Invalid JSON in config file
    ConfigInvalidJson(serde_json::error::Error),
    /// Vault name or file missing
    VaultNotFound(String),
    /// Vault name or path not specified
    VaultNotSpecified,
    /// Vault directory missing or path not pointing to a directory
    VaultDirNotFound(PathBuf),
    /// Vault file is corrupted
    VaultCorrupted,
    /// Wrong password (file can also be corrupted, but unlikely)
    WrongPassword,
    /// Vault already exists, duplicate vault names are not allowed
    VaultALreadyExists(String),
    /// Item already exists, duplicate item names are not allowed
    ItemALreadyExists(String),
    /// Item doesn't exist
    NoSuchItem(String),
    /// Input, path or filename contains non-unicode characters
    NonUnicodeInput,
    /// Path: Required directory, got file
    DirectoryRequired(PathBuf),
    /// Path: Required file, got directory
    FileRequired(PathBuf),
    /// Name format not allowed
    VaultNameInvalid(ValidationError),
    /// Vault folder not initialized
    NotInitialized,
    /// No password set for item
    ItemNoPasswordSet,
}
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}
