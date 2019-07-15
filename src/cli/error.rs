use std::io;
use std::path::PathBuf;

use super::validate::ValidationError;
use crate::backend::book::VersionMergeError;
use crate::sync;

#[must_use]
pub type VResult<T> = Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// Generic IO error
    Io(io::Error),
    /// Syncronization error
    Sync(sync::Error),
    /// Invalid JSON
    Json(serde_json::error::Error),
    /// Invalid bincode
    Bincode(bincode::ErrorKind),
    /// Invalid Base64
    Base64Decode(base64::DecodeError),
    /// Book version merge error
    BookVersionMergeError(VersionMergeError),
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
    VaultAlreadyExists(String),
    /// Item already exists, duplicate item names are not allowed
    ItemAlreadyExists(String),
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
    /// Synchronization transfer string not valid
    SynchronizationTransferString,
    /// Synchronization transfer string from an old version
    SynchronizationTransferStringVersion(u8, u8),
}
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}
impl From<serde_json::error::Error> for Error {
    fn from(error: serde_json::error::Error) -> Self {
        Error::Json(error)
    }
}
impl From<Box<bincode::ErrorKind>> for Error {
    fn from(error: Box<bincode::ErrorKind>) -> Self {
        Error::Bincode(*error)
    }
}
impl From<base64::DecodeError> for Error {
    fn from(error: base64::DecodeError) -> Self {
        Error::Base64Decode(error)
    }
}
impl From<sync::Error> for Error {
    fn from(error: sync::Error) -> Self {
        if let sync::Error::Io(ioerror) = error {
            Error::Io(ioerror)
        } else {
            Error::Sync(error)
        }
    }
}
impl From<VersionMergeError> for Error {
    fn from(error: VersionMergeError) -> Self {
        Error::BookVersionMergeError(error)
    }
}
