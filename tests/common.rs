#![allow(dead_code, unused_imports)]

use assert_cmd::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::process::Command;
use tempfile::{tempdir, TempDir};

#[macro_export]
macro_rules! cmd {
    ($td:expr; $($a:expr)*) => {{
        let output = Command::cargo_bin(env!("CARGO_PKG_NAME"))
            .unwrap()
            .args(&[$($a,)*])
            .env("VPASS_VAULT_DIR", $td.path())
            .env(concat!("RUST_LOG=", env!("CARGO_PKG_NAME")), "trace")
            .unwrap();
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        assert!(output.status.success());
    }};
}

#[macro_export]
macro_rules! cmd_stdout {
    ($td:expr; $($a:expr)*) => {{
        let output = Command::cargo_bin(env!("CARGO_PKG_NAME"))
            .unwrap()
            .args(&[$($a,)*])
            .env("VPASS_VAULT_DIR", $td.path())
            .env(concat!("RUST_LOG=", env!("CARGO_PKG_NAME")), "trace")
            .unwrap();
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        assert!(output.status.success());
        output.stdout
    }};
}

#[must_use]
pub fn init() -> io::Result<TempDir> {
    let td = tempdir().unwrap();
    cmd!(td; "init");
    Ok(td)
}

pub fn vault_create(td: &TempDir, name: &str, password: &str) {
    cmd!(td; "vault" "create" name "-p" password)
}

pub fn vault_rename(td: &TempDir, old_name: &str, new_name: &str, password: &str) {
    cmd!(td; "-p" password "vault" "rename" old_name new_name)
}

pub fn vault_delete(td: &TempDir, name: &str) {
    cmd!(td; "vault" "delete" name "--force")
}

pub fn vault_change_password(td: &TempDir, name: &str, old_password: &str, new_password: &str) {
    cmd!(td; "-p" old_password "vault" "change-password" name "-p" new_password)
}

pub fn check_password(td: &TempDir, name: &str, password: &str) {
    cmd!(td; "-p" password "-n" name "list")
}

pub fn add_item(td: &TempDir, name: &str, password: &str, item_name: &str, item_password: &str) {
    cmd!(td; "-p" password "-n" name "add" item_name "-p" item_password)
}

pub fn remove_item(td: &TempDir, name: &str, password: &str, item_name: &str) {
    cmd!(td; "-p" password "-n" name "remove" item_name)
}

pub fn edit_item_change_password(
    td: &TempDir, name: &str, password: &str, item_name: &str, new_password: &str,
) {
    cmd!(td; "-p" password "-n" name "edit" item_name "-p" new_password)
}

pub fn edit_item_add_tag(td: &TempDir, name: &str, password: &str, item_name: &str, tag: &str) {
    cmd!(td; "-p" password "-n" name "edit" item_name "-t" tag)
}

pub fn get_item_json(td: &TempDir, name: &str, password: &str, item_name: &str) -> serde_json::Value {
    let output = Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .unwrap()
        .args(&["-p", password, "-n", name, "show", item_name, "-jp"])
        .env("VPASS_VAULT_DIR", td.path())
        .unwrap();
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).unwrap()
}
