#![feature(trait_alias)]
#![feature(bind_by_move_pattern_guards)]
#![feature(vec_remove_item)]
#![feature(unsized_locals)]
#![feature(box_patterns)]
#![feature(box_syntax)]
#![deny(bare_trait_objects)]
#![deny(unused_must_use)]
#![warn(clippy::all)]
#![allow(dead_code, unused_imports)]

mod backend;
pub mod cli;
pub mod sync;

use std::fs;
use std::io;
use std::path::Path;

pub use backend::book::{Book, Item, ItemMetadata, Password};
use backend::vault::{EncryptedVault, Vault};
use cli::error::{Error, VResult};

/// Decrypt vault bytes to a book
pub fn decrypt(data: &[u8], password: &str) -> VResult<Book> {
    Ok(EncryptedVault::from_bytes(data)
        .map_err(|_| Error::VaultCorrupted)?
        .decrypt(password)
        .ok_or(Error::WrongPassword)?
        .content)
}

/// Encrypt a book to vault bytes
pub fn encrypt(password: &str, book: Book) -> VResult<Vec<u8>> {
    Ok(Vault::new(book).encrypt(password).to_bytes())
}

/// Read an encrypted book from a file
pub fn read(path: &Path, password: &str) -> VResult<Book> {
    Ok(EncryptedVault::from_bytes(&fs::read(path)?)
        .map_err(|_| Error::VaultCorrupted)?
        .decrypt(password)
        .ok_or(Error::WrongPassword)?
        .content)
}

/// Write a book to an encrypted file
pub fn write(path: &Path, password: &str, book: Book) -> VResult<()> {
    let encrypted = Vault::new(book).encrypt(password);
    fs::write(path, encrypted.to_bytes()).map_err(Error::from)
}

/// Creates a new, empty vault to given path
pub fn create(path: &Path, password: &str) -> VResult<()> {
    if path.exists() {
        Err(Error::from(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Vault already exists",
        )))
    } else {
        write(path, password, Book::new())
    }
}
