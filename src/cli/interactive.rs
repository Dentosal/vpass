use crate::backend::book::Book;
use crate::sync::config::{self, SyncConfig};
use crate::sync::providers::Provider;
use crate::{Error, VResult};

use std::fmt;
use std::io::prelude::*;
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde_json::Value;
use strum::IntoEnumIterator;

pub fn prompt_string(prompt: &str) -> VResult<String> {
    let mut buf = String::new();
    loop {
        print!("{}: ", prompt);
        std::io::stdout().flush().unwrap();
        std::io::stdin().lock().read_line(&mut buf).unwrap();
        buf = buf.trim().to_owned();
        if !buf.is_empty() {
            return Ok(buf);
        }
        println!("Non-empty answer required");
        buf.clear();
    }
}

pub fn prompt_password(prompt: &str) -> VResult<String> {
    loop {
        let pass = rpassword::read_password_from_tty(Some(&format!("{}: ", prompt)))?;
        println!();
        if pass != "" {
            return Ok(pass);
        }
        println!("Non-empty password required");
    }
}

pub fn prompt_boolean(prompt: &str) -> VResult<bool> {
    let mut buf = String::new();
    loop {
        print!("{} [y/n]: ", prompt);
        std::io::stdout().flush().unwrap();
        std::io::stdin().lock().read_line(&mut buf).unwrap();
        buf = buf.trim().to_owned();
        if buf == "y" || buf == "yes" {
            return Ok(true);
        } else if buf == "n" || buf == "no" {
            return Ok(false);
        }
        println!("Invalid option '{}'", buf.trim());
        buf.clear();
    }
}

pub fn prompt_enum<E: FromStr + IntoEnumIterator>(prompt: &str) -> VResult<E>
where
    <E as IntoEnumIterator>::Iterator: Iterator,
    <<E as IntoEnumIterator>::Iterator as Iterator>::Item: fmt::Display,
{
    let mut buf = String::new();
    println!("{}:", prompt);
    for variant in E::iter() {
        println!("* {}", variant);
    }
    loop {
        print!("> ");
        std::io::stdout().flush().unwrap();
        std::io::stdin().lock().read_line(&mut buf).unwrap();
        if let Ok(v) = buf.trim().parse::<E>() {
            return Ok(v);
        }
        println!("Invalid option '{}'", buf.trim());
        buf.clear();
    }
}

pub fn prompt_dir_path(prompt: &str) -> VResult<PathBuf> {
    let mut buf = String::new();
    loop {
        print!("{}: ", prompt);
        std::io::stdout().flush().unwrap();
        std::io::stdin().lock().read_line(&mut buf).unwrap();
        let p = Path::new(buf.trim());
        if p.exists() {
            return Ok(p.to_owned());
        }
        println!("Directory not found {:?}", p);
        buf.clear();
    }
}

/// Returns `Ok(None)` if cancelled
pub fn sync_setup(book: &Book) -> VResult<Option<SyncConfig>> {
    if let Some(c) = config::book_read(book)? {
        println!("Synchronization is already configured ({})", c.service);
        if !prompt_boolean("Overwrite?")? {
            return Ok(None);
        }
    }

    let p = prompt_enum::<Provider>("Select a provider")?;
    let data: Value = p.interactive_setup()?;
    let mut service = p.load(&data);
    service.ping()?;
    Ok(Some(SyncConfig { service: p, data }))
}
