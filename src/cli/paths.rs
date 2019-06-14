use std::path::PathBuf;

use super::opt;

pub fn data_dir(args: &opt::OptRoot) -> PathBuf {
    if let Some(ref v) = args.vault_dir {
        v.into()
    } else if cfg!(windows) {
        dirs::data_dir().unwrap().join(env!("CARGO_PKG_NAME"))
    } else {
        dirs::home_dir()
            .unwrap()
            .join(concat!(".", env!("CARGO_PKG_NAME")))
    }
}
pub fn config_file(args: &opt::OptRoot) -> PathBuf {
    if args.disable_config {
        panic!("Config file use disabled");
    } else if let Some(cpath) = args.config.clone() {
        cpath
    } else {
        data_dir(args).join("config.json")
    }
}
