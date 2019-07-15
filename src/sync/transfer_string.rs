use super::config::SyncConfig;
use super::providers::Provider;

use crate::cli::{Error, VResult};

use bitvec::vec::BitVec;
use crc::{crc16, Hasher16};
use serde::{Deserialize, Serialize};
use std::mem::size_of;

const PREFIX: &str = "VPASS_";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
struct Metadata {
    major: u8,
    minor: u8,
    crc16: u16,
}
impl Metadata {
    fn check(&self, data: &[u8]) -> VResult<()> {
        let cmp = Self::new(data);
        if cmp.major != self.major && cmp.minor == self.minor {
            Err(Error::SynchronizationTransferStringVersion(cmp.major, cmp.minor))
        } else if cmp.crc16 != self.crc16 {
            Err(Error::SynchronizationTransferString)
        } else {
            Ok(())
        }
    }

    fn new(data: &[u8]) -> Self {
        Self {
            major: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
            minor: env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
            crc16: crc16::checksum_x25(data),
        }
    }
}

pub fn encode(c: &SyncConfig) -> String {
    let data = c.compress();
    let meta = Metadata::new(&data);
    let mut compressed = bincode::serialize(&meta).unwrap();
    compressed.extend(data);
    format!("{}{}", PREFIX, base64::encode(&compressed))
}

pub fn decode(s: &str) -> VResult<SyncConfig> {
    if !s.starts_with(PREFIX) {
        return Err(Error::SynchronizationTransferString);
    }

    let decoded = base64::decode(&s[PREFIX.len()..])?;

    let meta_size = size_of::<Metadata>();
    let meta: Metadata = bincode::deserialize(&decoded[..meta_size])?;
    meta.check(&decoded[meta_size..])?;
    Ok(SyncConfig::decompress(&decoded[meta_size..])?)
}

#[cfg(test)]
mod tests {
    use super::super::config::SyncConfig;
    use super::super::providers::Provider;
    use super::{decode, encode, Metadata};
    use crate::cli::VResult;

    #[test]
    fn metadata_serialized_size() {
        let m = Metadata::new(&vec![1; 100]);
        assert_eq!(
            bincode::serialize(&m).unwrap().len(),
            std::mem::size_of::<Metadata>()
        );
    }

    #[test]
    fn encode_decode() {
        let c = SyncConfig {
            service: Provider::Mock,
            data: serde_json::Value::Null,
        };

        assert_eq!(decode(&encode(&c)).expect("Decode failed"), c);
    }

    #[test]
    #[should_panic]
    fn decode_invalid_prefix() {
        decode("RANDOM INVALID STRING").expect("Decode failed");
    }

    #[test]
    #[should_panic]
    fn decode_invalid() {
        decode("VPASS_abcd").expect("Decode failed");
    }
}
