use assert_cmd::prelude::*;
use std::fs;
use std::io;
use std::process::Command;
use tempfile::{tempdir, TempDir};

#[test]
fn test_cli_version() {
    let output = Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .unwrap()
        .args(&["-V"])
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        format!("{} {}\n", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        String::from_utf8(output.stdout).unwrap()
    );
    assert_eq!("", String::from_utf8(output.stderr).unwrap());
}

#[must_use]
fn init() -> io::Result<TempDir> {
    let td = tempdir().unwrap();
    let output = Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .unwrap()
        .args(&["init"])
        .env("VPASS_VAULT_DIR", td.path())
        .unwrap();
    assert!(output.status.success());
    Ok(td)
}

macro_rules! cmd {
    ($td:expr; $($a:expr)*) => {{
        let output = Command::cargo_bin(env!("CARGO_PKG_NAME"))
            .unwrap()
            .args(&[$($a,)*])
            .env("VPASS_VAULT_DIR", $td.path())
            .unwrap();
        assert!(output.status.success());
    }};
}

fn vault_create(td: &TempDir, name: &str, password: &str) {
    cmd!(td; "-p" password "vault" "create" name)
}

fn vault_rename(td: &TempDir, old_name: &str, new_name: &str) {
    cmd!(td; "vault" "rename" old_name new_name)
}

fn vault_delete(td: &TempDir, name: &str) {
    cmd!(td; "vault" "delete" name "--force")
}

fn vault_change_password(td: &TempDir, name: &str, old_password: &str, new_password: &str) {
    cmd!(td; "-p" old_password "vault" "change-password" name "-p" new_password)
}

fn check_password(td: &TempDir, name: &str, password: &str) {
    cmd!(td; "-p" password "-n" name "list")
}

fn add_item(td: &TempDir, name: &str, password: &str, item_name: &str, item_password: &str) {
    cmd!(td; "-p" password "-n" name "add" item_name "-p" item_password)
}

fn get_item_json(td: &TempDir, name: &str, password: &str, item_name: &str) -> serde_json::Value {
    let output = Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .unwrap()
        .args(&["-p", password, "-n", name, "show", item_name, "-jp"])
        .env("VPASS_VAULT_DIR", td.path())
        .unwrap();
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn test_cli_init() -> io::Result<()> {
    use vpass::cli::cfg::Config;

    let td = init()?;
    let c = fs::read(td.path().join("config.json"))?;
    assert_eq!(c, Config::default().to_json_bytes());
    Ok(())
}

#[test]
fn test_vault_ops() -> io::Result<()> {
    let td = init()?;
    vault_create(&td, "test1", "password");
    vault_create(&td, "test2", "password");
    vault_create(&td, "test3", "password2");

    let t1 = fs::read(td.path().join("test1.vpass_vault"))?;
    let t2 = fs::read(td.path().join("test2.vpass_vault"))?;
    assert_ne!(t1, t2, "Equal nonce or password salt shouldn't occur");

    let t3 = fs::read(td.path().join("test3.vpass_vault"))?;
    vault_rename(&td, "test3", "test4");
    let t4 = fs::read(td.path().join("test4.vpass_vault"))?;
    assert_eq!(t3, t4, "Rename should not modify vault");

    assert!(td.path().join("test4.vpass_vault").exists());
    vault_delete(&td, "test4");
    assert!(!td.path().join("test4.vpass_vault").exists());

    Ok(())
}

#[test]
#[should_panic]
fn test_vault_delete_nonexistent() {
    let td = init().unwrap();
    vault_delete(&td, "test4");
}

#[test]
fn test_vault_change_password() -> io::Result<()> {
    let td = init()?;
    vault_create(&td, "test1", "password");
    vault_change_password(&td, "test1", "password", "new_password");
    check_password(&td, "test1", "new_password");
    Ok(())
}

#[test]
#[should_panic]
fn test_vault_wrong_password() {
    let td = init().unwrap();
    vault_create(&td, "test1", "p1");
    check_password(&td, "test1", "p2");
}

#[test]
#[should_panic]
fn test_vault_create_duplicate() {
    let td = init().unwrap();
    vault_create(&td, "test1", "p1");
    vault_create(&td, "test1", "p2");
}

#[test]
#[should_panic]
fn test_vault_rename_duplicate() {
    let td = init().unwrap();
    vault_create(&td, "test1", "p1");
    vault_create(&td, "test2", "p2");
    vault_rename(&td, "test1", "test2");
}

#[test]
fn test_vault_new_item() -> io::Result<()> {
    let td = init()?;
    vault_create(&td, "test", "password");
    add_item(&td, "test", "password", "item_name", "item_password");
    let json = get_item_json(&td, "test", "password", "item_name");
    let pw = json.as_object().unwrap().get("password").unwrap();
    assert_eq!(pw, "item_password");
    Ok(())
}
