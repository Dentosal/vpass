use std::path::PathBuf;

use super::{error::*, opt::OptRoot};

pub fn data_dir(args: &OptRoot) -> VResult<PathBuf> {
    if let Some(ref v) = args.vault_dir {
        Ok(v.into())
    } else if cfg!(windows) {
        Ok(dirs::data_dir().unwrap().join(env!("CARGO_PKG_NAME")))
    } else {
        Ok(dirs::home_dir()
            .expect("Home directory not available")
            .join(concat!(".", env!("CARGO_PKG_NAME"))))
    }
}
pub fn config_file(args: &OptRoot) -> VResult<PathBuf> {
    if args.disable_config {
        panic!("Config file use disabled");
    } else if let Some(cpath) = args.config.clone() {
        Ok(cpath)
    } else {
        Ok(data_dir(args)?.join("config.json"))
    }
}
