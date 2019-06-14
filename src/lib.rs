#![feature(trait_alias)]
#![feature(bind_by_move_pattern_guards)]
#![feature(vec_remove_item)]
#![deny(unused_must_use)]
#![deny(clippy::all)]
#![allow(dead_code, unused_imports)]

mod backend;
pub mod cli;

use std::fs;
use std::io;
use std::path::Path;

pub use backend::book::{Book, Item, ItemMetadata, Password};
use backend::vault::{EncryptedVault, Vault};

/// Read an encrypted book from a file
#[must_use]
pub fn read(path: &Path, password: &str) -> Option<Book> {
    let vault = EncryptedVault::from_bytes(&fs::read(path).ok()?)
        .decrypt(password)
        .expect("Wrong password or corrupted file");
    Some(vault.content)
}

/// Write a book to an encrypted file
#[must_use]
pub fn write(path: &Path, password: &str, book: Book) -> io::Result<()> {
    let encrypted = Vault::new(book).encrypt(password);
    fs::write(path, encrypted.to_bytes())
}

/// Creates a new, empty vault to given path
#[must_use]
pub fn create_vault(path: &Path, password: &str) -> io::Result<()> {
    if path.exists() {
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Vault already exists",
        ))
    } else {
        write(path, password, Book::new())
    }
}
