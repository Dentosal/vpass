use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    if cfg!(windows) {
        dirs::data_dir().unwrap().join(env!("CARGO_PKG_NAME"))
    } else {
        dirs::home_dir()
            .unwrap()
            .join(concat!(".", env!("CARGO_PKG_NAME")))
    }
}
pub fn config_file() -> PathBuf {
    data_dir().join("config.json")
}
