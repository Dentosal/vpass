#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fs;

use super::{error::*, opt, paths};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Config {
    pub default_vault: Option<String>,
}
impl Config {
    pub const fn default() -> Self {
        Self { default_vault: None }
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }

    pub fn to_json_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }

    pub fn from_json_bytes(data: &[u8]) -> VResult<Self> {
        serde_json::from_slice(data).map_err(|e| Error::ConfigInvalidJson(e))
    }
}

pub fn read(args: &opt::OptRoot) -> VResult<Config> {
    if args.disable_config {
        Ok(Config::default())
    } else {
        let p = paths::config_file(args)?;
        Config::from_json_bytes(&fs::read(p)?)
    }
}

pub fn write(args: &opt::OptRoot, c: Config) -> VResult<()> {
    let p = paths::config_file(args)?;
    fs::write(p, c.to_json_bytes())?;
    Ok(())
}

pub fn modify<F, R>(args: &opt::OptRoot, f: F) -> VResult<R>
where
    F: FnOnce(&mut Config) -> R,
{
    let mut c = read(args)?;
    let r = f(&mut c);
    write(args, c)?;
    Ok(r)
}
