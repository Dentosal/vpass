use assert_cmd::prelude::*;
use serde_json::json;
use std::fs;
use std::io;
use std::process::Command;
use tempfile::{tempdir, TempDir};

mod common;
use common::*;

#[must_use]
fn create_sync_fs() -> io::Result<TempDir> {
    let td = tempdir().unwrap();
    fs::write(td.path().join("VPassFile"), &[])?;
    Ok(td)
}

#[test]
fn test_sync_push() -> io::Result<()> {
    let td = init()?;
    let td_sync = create_sync_fs()?;

    vault_create(&td, "testvault", "password");
    cmd!(td; "-n" "testvault" "-p" "password" "sync" "setup"
        "--json" format!("{{\"service\":\"FileSystem\",\"data\":{{\"path\":{:?}}}}}", td_sync.path()).as_str()
    );

    cmd!(td; "-n" "testvault" "-p" "password" "sync");

    assert!(td.path().join("testvault.vpass_vault").exists());
    assert!(td_sync.path().join("VPassFile").exists());
    assert!(td_sync.path().join("testvault.vpass_vault").exists());

    Ok(())
}

#[test]
fn test_sync_import_data() -> io::Result<()> {
    let td = init()?;
    let td_sync = create_sync_fs()?;

    vault_create(&td, "testvault", "password");
    cmd!(td; "-n" "testvault" "-p" "password" "sync" "setup"
        "--json" json!({
            "service": "FileSystem",
            "data": {
                "path": td_sync.path()
            }
        }).to_string().as_str()
    );

    cmd!(td; "-n" "testvault" "-p" "password" "add" "testitem" "-p" "testpassword");
    cmd!(td; "-n" "testvault" "-p" "password" "sync");
    let import_string = cmd_stdout!(td; "-n" "testvault" "-p" "password" "sync" "export");
    assert!(import_string.starts_with(b"VPASS_"));
    vault_delete(&td, "testvault");

    cmd!(td; "-p" "password" "vault" "import" "testvault" String::from_utf8(import_string).unwrap().as_str().trim());

    let data = get_item_json(&td, "testvault", "password", "testitem");
    assert_eq!(
        data.get("password").and_then(serde_json::Value::as_str),
        Some("testpassword")
    );

    Ok(())
}
