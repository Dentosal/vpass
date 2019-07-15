//! Uses a filesystem folder as a repository.
//! An empty file called VPassFile is used to mark this as a vpass repository.

use super::super::{Error, SyncProvider, SyncResult, UpdateKey};
use crate::VResult;

use base64;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

use log::debug;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    path: PathBuf,
}
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "FileSystem_Config({:?})", self.path)
    }
}

pub struct FileSystem {
    config: Config,
}
impl SyncProvider for FileSystem {
    fn interactive_setup() -> VResult<Value> {
        use crate::cli::interactive::*;

        let c = Config {
            path: prompt_dir_path("Directory to use")?,
        };

        fs::write(c.path.join("VPassFile"), &[])?;
        Ok(serde_json::to_value(&c).unwrap())
    }

    fn load(value: &Value) -> Self
    where Self: Sized {
        FileSystem {
            config: serde_json::from_value(value.clone()).expect("Invalid config for FileSystem integration"),
        }
    }

    fn ping(&mut self) -> SyncResult<()> {
        if self.config.path.exists() {
            Ok(())
        } else {
            Err(Error::InvalidRemote)
        }
    }

    fn test(&mut self) -> SyncResult<()> {
        if !self.config.path.join("VPassFile").exists() {
            return Err(Error::InvalidRemote);
        }
        Ok(())
    }

    fn create(&mut self, key: &str, value: Vec<u8>) -> SyncResult<()> {
        debug!("Create: {}", key);

        fs::write(self.config.path.join(key), &value)?;
        Ok(())
    }

    fn update(&mut self, key: &str, value: Vec<u8>, update_key: UpdateKey) -> SyncResult<()> {
        debug!("Update: {}", key);
        let (_, uk) = self.read(key)?;
        if uk != update_key {
            Err(Error::InvalidUpdateKey)
        } else {
            fs::write(self.config.path.join(key), &value)?;
            Ok(())
        }
    }

    fn read(&mut self, key: &str) -> SyncResult<(Vec<u8>, UpdateKey)> {
        debug!("Read: {}", key);
        if !self.config.path.join(key).exists() {
            return Err(Error::NoSuchKey(key.to_owned()));
        }
        let data = fs::read(self.config.path.join(key))?;
        let uk = UpdateKey::from_byte(data.get(0).copied().unwrap_or(0));
        Ok((data, uk))
    }

    fn delete(&mut self, key: &str, update_key: UpdateKey) -> SyncResult<()> {
        debug!("Delete: {}", key);
        let (_, uk) = self.read(key)?;
        if uk != update_key {
            Err(Error::InvalidUpdateKey)
        } else {
            fs::remove_file(self.config.path.join(key))?;
            Ok(())
        }
    }
}
