pub mod filesystem;
pub mod github;
pub mod mock;

use super::SyncProvider;
use crate::VResult;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum_macros::{Display, EnumIter, EnumProperty, EnumString};

#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Display,
    EnumIter,
    EnumString,
    EnumProperty,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
#[repr(u8)]
pub enum Provider {
    #[strum(props(tag = "0"))]
    GitHub,
    #[strum(props(tag = "1"))]
    FileSystem,
    #[strum(props(tag = "2"))]
    Mock,
}
impl Provider {
    pub fn load(self, value: &Value) -> Box<dyn SyncProvider> {
        match self {
            Self::GitHub => Box::new(github::GitHub::load(value)) as Box<dyn SyncProvider>,
            Self::FileSystem => Box::new(filesystem::FileSystem::load(value)) as Box<dyn SyncProvider>,
            Self::Mock => Box::new(mock::Mock::load(value)) as Box<dyn SyncProvider>,
        }
    }

    pub fn interactive_setup(self) -> VResult<Value> {
        match self {
            Self::GitHub => github::GitHub::interactive_setup(),
            Self::FileSystem => filesystem::FileSystem::interactive_setup(),
            Self::Mock => mock::Mock::interactive_setup(),
        }
    }

    pub fn configuration_compress(self, v: &Value) -> Vec<u8> {
        match self {
            Self::GitHub => github::GitHub::configuration_compress(v),
            Self::FileSystem => filesystem::FileSystem::configuration_compress(v),
            Self::Mock => mock::Mock::configuration_compress(v),
        }
    }

    pub fn configuration_decompress(self, v: &[u8]) -> VResult<Value> {
        match self {
            Self::GitHub => github::GitHub::configuration_decompress(v),
            Self::FileSystem => filesystem::FileSystem::configuration_decompress(v),
            Self::Mock => mock::Mock::configuration_decompress(v),
        }
    }
}
