//! Stores files in memory instead of actually saving them somewhere.
//! All data is discarded when the service is unloaded.
//! Used for testing, and maybe for dry-runs in the future.

use super::super::{Error, SyncProvider, SyncResult, UpdateKey};
use crate::VResult;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

pub struct Mock {
    items: HashMap<String, Vec<u8>>,
}
impl SyncProvider for Mock {
    fn interactive_setup() -> VResult<Value> {
        Ok(Value::Null)
    }

    fn load(_: &Value) -> Self
    where Self: Sized {
        Mock {
            items: HashMap::new(),
        }
    }

    fn ping(&mut self) -> SyncResult<()> {
        Ok(())
    }

    fn test(&mut self) -> SyncResult<()> {
        Ok(())
    }

    fn create(&mut self, key: &str, value: Vec<u8>) -> SyncResult<()> {
        self.items.insert(key.to_owned(), value);
        Ok(())
    }

    fn update(&mut self, key: &str, value: Vec<u8>, update_key: UpdateKey) -> SyncResult<()> {
        let (_, uk) = self.read(key)?;
        assert_eq!(uk, update_key, "Incorrect update key");
        self.items.insert(key.to_owned(), value);
        Ok(())
    }

    fn read(&mut self, key: &str) -> SyncResult<(Vec<u8>, UpdateKey)> {
        let data = self
            .items
            .get(key)
            .ok_or_else(|| Error::NoSuchKey(key.to_owned()))?;
        Ok((
            data.clone(),
            UpdateKey::from_byte(data.get(data.len() - 1).copied().unwrap_or(0)),
        ))
    }

    fn delete(&mut self, key: &str) -> SyncResult<()> {
        self.items.remove(key);
        Ok(())
    }
}
