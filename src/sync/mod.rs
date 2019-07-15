//! Synchronization abstraction layer,
//! provides any sync service as key-value store.
//! Metavault `sync.meta.vpass_vault` is used for storing
//! per-service configurations and access keys.

pub mod config;
mod error;
pub mod providers;
pub mod transfer_string;

use crate::{backend::book::Item, backend::book::Password, Book, VResult};

use log::debug;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub use self::error::Error;

#[must_use]
pub type SyncResult<T> = Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateKey(Vec<u8>);
impl UpdateKey {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(bytes.to_vec())
    }

    pub fn from_byte(byte: u8) -> Self {
        Self(vec![byte])
    }

    pub fn from_string(s: String) -> Self {
        Self(s.bytes().collect())
    }

    pub fn to_string(&self) -> String {
        String::from_utf8(self.0.clone()).expect("UpdateKey is not a string")
    }
}

pub trait SyncProvider {
    /// Interactive configuration setup.
    fn interactive_setup() -> VResult<Value>
    where Self: Sized;

    /// Custom compression for configuration.
    /// Provider should overwrite this to get smaller transfer strings.
    fn configuration_compress(item: &Value) -> Vec<u8>
    where Self: Sized {
        serde_json::to_vec(&item).unwrap()
    }

    /// Custom decompression for configuration.
    /// Provider should overwrite this to get smaller transfer strings.
    fn configuration_decompress(data: &[u8]) -> VResult<Value>
    where Self: Sized {
        Ok(serde_json::from_slice(&data)?)
    }

    /// Load from config item from persistent configuration.
    /// Panics if configuration has invalid format.
    fn load(item: &Value) -> Self
    where Self: Sized;

    /// Check access and authentication.
    /// Can also act as a session refresh heartbeat.
    fn ping(&mut self) -> SyncResult<()>;

    /// Run sanity checks on the remote.
    /// Ping is always called successfully before this.
    fn test(&mut self) -> SyncResult<()>;

    /// Create a new key and set value
    fn create(&mut self, key: &str, value: Vec<u8>) -> SyncResult<()>;

    /// Update value to key, overriding the previous value
    fn update(&mut self, key: &str, value: Vec<u8>, update_key: UpdateKey) -> SyncResult<()>;

    /// Read value by key. Also returns UpdateKey to update.
    fn read(&mut self, key: &str) -> SyncResult<(Vec<u8>, UpdateKey)>;

    /// Check if key exists
    fn exists(&mut self, key: &str) -> SyncResult<bool> {
        match self.read(key) {
            Ok(_) => Ok(true),
            Err(Error::NoSuchKey(_)) => Ok(false),
            e => e.map(|_| unreachable!()),
        }
    }

    /// Read value by key
    fn delete(&mut self, key: &str) -> SyncResult<()>;
}

fn load_service(book: &Book) -> VResult<Option<Box<dyn SyncProvider>>> {
    if let Some(cfg) = config::book_read(&book)? {
        // Load service information
        let mut service = cfg.service.load(&cfg.data);
        // Check credentials and internet access
        (*service).ping()?;
        // Run sanity checks for the remote
        service.test()?;
        // Everything ok, return the instance
        Ok(Some(service))
    } else {
        Ok(None)
    }
}

/// Check if some other key on the same sync repository is free.
/// Checked before renaming vaults on remote.
/// Unsynchronized books always return Ok.
pub fn check_rename(new_key: &str, book: &Book) -> VResult<()> {
    if let Some(mut service) = load_service(book)? {
        if (*service).exists(new_key)? {
            return Err(Error::KeyAlreadyExists(new_key.to_owned()).into());
        }
    }
    Ok(())
}

/// Pushes new vault to remote, causes error if it already exists.
/// Unsynchronized books are skipped with Ok.
pub fn create(key: &str, book: &Book, password: &str) -> VResult<()> {
    if let Some(mut service) = load_service(book)? {
        if (*service).exists(key)? {
            return Err(Error::KeyAlreadyExists(key.to_owned()).into());
        }
        let data = crate::encrypt(password, book.clone())?;
        service.create(key, data)?;
    }
    Ok(())
}

/// Downloads a book from remote
pub fn download_book(key: &str, c: config::SyncConfig, password: &str) -> VResult<Book> {
    let mut service = c.service.load(&c.data);
    (*service).ping()?;
    (*service).test()?;

    let (vault_data, _) = (*service).read(key)?;
    crate::decrypt(&vault_data, password)
}

/// Synchronizes local changes to remote.
/// Unsynchronized books are skipped with Ok.
pub fn vault(key: &str, book: &mut Book, password: &str) -> VResult<()> {
    if let Some(service) = load_service(book)? {
        synchronize(*service, key, book, password)
    } else {
        Ok(())
    }
}

/// Force pushes local changes to remote.
/// Doesn't even check if they have same origin.
/// Unsynchronized books are skipped with Ok.
pub fn vault_overwrite(key: &str, book: &Book, password: &str) -> VResult<()> {
    if let Some(service) = load_service(book)? {
        synchronize_overwrite(*service, key, book, password)
    } else {
        Ok(())
    }
}

/// Delete a vault from the remote.
pub fn delete(key: &str, book: &Book) -> VResult<()> {
    if let Some(mut service) = load_service(book)? {
        (*service).delete(key)?;
    }
    Ok(())
}

fn synchronize(mut sp: dyn SyncProvider, key: &str, book: &mut Book, password: &str) -> VResult<()> {
    match sp.read(key) {
        Ok((old_data, update_key)) => {
            let b_old = crate::decrypt(&old_data, password)?;
            if b_old != *book {
                // Book is not updated until the new version is actually synchronized,
                // so that this function is atomic regarding version merges.
                // If pushing the new version fails, the local book is still in the original state.
                let b_new = b_old.merge_versions(book)?;
                let data = crate::encrypt(password, b_new.clone())?;
                sp.update(key, data, update_key)?;
                *book = b_new;
            }
            Ok(())
        },
        Err(Error::NoSuchKey(_)) => {
            let data = crate::encrypt(password, book.clone())?;
            sp.create(key, data)?;
            Ok(())
        },
        e => {
            e?;
            unreachable!()
        },
    }
}

/// Synchronize vault, overwriting the old value.
fn synchronize_overwrite(mut sp: dyn SyncProvider, key: &str, book: &Book, password: &str) -> VResult<()> {
    let data = crate::encrypt(password, book.clone())?;
    match sp.read(key) {
        Ok((_, update_key)) => {
            sp.update(key, data, update_key)?;
            Ok(())
        },
        Err(Error::NoSuchKey(_)) => {
            sp.create(key, data)?;
            Ok(())
        },
        e => {
            e?;
            unreachable!()
        },
    }
}
