use assert_cmd::prelude::*;
use maplit::hashset;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::process::Command;

mod common;
use common::*;

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
    vault_rename(&td, "test3", "test4", "password2");
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
    vault_rename(&td, "test1", "test2", "p1");
}

#[test]
fn test_new_item() -> io::Result<()> {
    let td = init()?;
    vault_create(&td, "test", "password");
    add_item(&td, "test", "password", "item_name", "item_password");
    let json = get_item_json(&td, "test", "password", "item_name");
    let pw = json.as_object().unwrap().get("password").unwrap();
    assert_eq!(pw, "item_password");
    Ok(())
}

#[test]
fn test_remove_item() -> io::Result<()> {
    let td = init()?;
    vault_create(&td, "test", "password");
    add_item(&td, "test", "password", "item_name", "item_password");
    remove_item(&td, "test", "password", "item_name");
    Ok(())
}

#[test]
fn test_remove_readd_item() -> io::Result<()> {
    let td = init()?;
    vault_create(&td, "test", "password");
    add_item(&td, "test", "password", "item_name", "item_password");
    remove_item(&td, "test", "password", "item_name");
    add_item(&td, "test", "password", "item_name", "item_password2");
    let json = get_item_json(&td, "test", "password", "item_name");
    let pw = json.as_object().unwrap().get("password").unwrap();
    assert_eq!(pw, "item_password2");
    Ok(())
}

#[test]
fn test_edit_item_password() -> io::Result<()> {
    let td = init()?;
    vault_create(&td, "test", "password");
    add_item(&td, "test", "password", "item_name", "item_password");
    edit_item_change_password(&td, "test", "password", "item_name", "new_password");
    let json = get_item_json(&td, "test", "password", "item_name");
    let pw = json.as_object().unwrap().get("password").unwrap();
    assert_eq!(pw, "new_password");
    Ok(())
}

#[test]
fn test_edit_item_tags() -> io::Result<()> {
    let td = init()?;
    vault_create(&td, "test", "password");
    add_item(&td, "test", "password", "item_name", "item_password");
    edit_item_add_tag(&td, "test", "password", "item_name", "tag1");
    edit_item_add_tag(&td, "test", "password", "item_name", "tag2");
    let json = get_item_json(&td, "test", "password", "item_name");
    let tag_values = json.as_object().unwrap().get("tags").unwrap();
    let tags = tag_values
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap().to_owned())
        .collect::<HashSet<_>>();
    assert_eq!(tags, hashset!["tag1".to_owned(), "tag2".to_owned()]);
    Ok(())
}
