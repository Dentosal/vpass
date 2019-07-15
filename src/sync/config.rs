use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use super::providers::Provider;
use super::Error;
use crate::{backend::book::Item, backend::book::Password, Book, Error as VError, VResult};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SyncConfig {
    /// Provider service
    pub service: Provider,
    /// Per-service persistent configuration data
    pub data: serde_json::Value,
    // TODO: Multiple services? services: Vec<ServiceConfig>
    // TODO: Allow write-only providers for backups? push_only: bool
    // TODO: Change remote key name to be diffrent than the name of the book filename?
}
impl SyncConfig {
    pub fn compress(&self) -> Vec<u8> {
        use strum::EnumProperty;
        let tag: u8 = self.service.get_str("tag").unwrap().parse().unwrap();
        let mut result = vec![tag];
        let data = self.service.configuration_compress(&self.data);
        result.extend(data);
        result
    }

    pub fn decompress(data: &[u8]) -> VResult<Self> {
        use strum::{EnumProperty, IntoEnumIterator};

        for service in Provider::iter() {
            if service.get_str("tag").unwrap().parse::<u8>().unwrap() == data[0] {
                return Ok(Self {
                    service,
                    data: service.configuration_decompress(&data[1..])?,
                });
            }
        }
        Err(VError::SynchronizationTransferString)
    }
}

const ITEM_NAME_SYNC_CONFIG: &str = "vpass/sync_config.json";

/// Read synchronization configuration from a book
pub fn book_read(book: &Book) -> VResult<Option<SyncConfig>> {
    if !book.has_item(ITEM_NAME_SYNC_CONFIG) {
        debug!("Syncronization configuration not set for the book");
        Ok(None)
    } else {
        let item = book.get_item_by_name(ITEM_NAME_SYNC_CONFIG).unwrap();
        let config_data = item.password.ok_or(Error::ConfigurationItem)?.plaintext();
        Ok(Some(
            serde_json::from_str(&config_data).map_err(|_| Error::ConfigurationItem)?,
        ))
    }
}

/// Remove synchronization configuration from a book
pub fn book_remove(book: &mut Book) -> VResult<()> {
    // Only error Book::remove can give is missing file, which is ok here
    let _ = book.remove(ITEM_NAME_SYNC_CONFIG);
    Ok(())
}

/// Setup synchronization configuration for a book
pub fn book_setup(book: &mut Book, cfg: SyncConfig) -> VResult<()> {
    // Verify config validity
    let mut service = cfg.service.load(&cfg.data);
    // Check credentials
    (*service).ping()?;
    // Actually write to the book
    let mut item = Item::new(ITEM_NAME_SYNC_CONFIG);
    item.password = Some(Password::new(&serde_json::to_string(&cfg).unwrap()));
    book.add(item)?;
    Ok(())
}
