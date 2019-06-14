#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fs;

use super::{opt, paths};

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

    pub fn from_json_bytes(data: &[u8]) -> Self {
        serde_json::from_slice(data).expect("Invalid JSON in config")
    }
}

pub fn read(args: &opt::OptRoot) -> Config {
    if args.disable_config {
        Config::default()
    } else {
        Config::from_json_bytes(&fs::read(paths::config_file(args)).expect("Unable to read config file"))
    }
}

pub fn write(args: &opt::OptRoot, c: Config) {
    fs::write(paths::config_file(args), c.to_json_bytes()).expect("Unable to write config");
}

pub fn modify<F, R>(args: &opt::OptRoot, f: F) -> R
where
    F: FnOnce(&mut Config) -> R,
{
    let mut c = read(args);
    let r = f(&mut c);
    write(args, c);
    r
}
