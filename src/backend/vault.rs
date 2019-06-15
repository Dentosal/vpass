use rust_sodium::{
    self,
    crypto::{pwhash, secretbox},
};

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fmt;
use std::io::prelude::*;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json;

pub trait Content = fmt::Debug + Serialize + DeserializeOwned + Clone + PartialEq + Eq;

const MAGIC: u8 = 0xd7;
const VERSION: u32 = 0;

/// Vault encryption/decryption key+salt from password
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
struct VaultKey {
    key: secretbox::Key,
    salt: pwhash::Salt,
}
impl VaultKey {
    pub fn new(password: &str) -> VaultKey {
        let salt = pwhash::gen_salt();
        let mut key = secretbox::Key([0; secretbox::KEYBYTES]);
        {
            let secretbox::Key(ref mut kb) = key;
            pwhash::derive_key(
                kb,
                password.as_bytes(),
                &salt,
                pwhash::OPSLIMIT_INTERACTIVE,
                pwhash::MEMLIMIT_INTERACTIVE,
            )
            .unwrap();
        }

        VaultKey { key, salt }
    }

    pub fn reconstruct(password: &str, salt: pwhash::Salt) -> VaultKey {
        let mut key = secretbox::Key([0; secretbox::KEYBYTES]);
        {
            let secretbox::Key(ref mut kb) = key;
            pwhash::derive_key(
                kb,
                password.as_bytes(),
                &salt,
                pwhash::OPSLIMIT_INTERACTIVE,
                pwhash::MEMLIMIT_INTERACTIVE,
            )
            .unwrap();
        }

        VaultKey { key, salt }
    }
}
impl fmt::Debug for VaultKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "VaultKey {{ key: ****, salt: {:?} }}", self.salt)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Vault<T> {
    pub content: T,
}
impl<T: Content> Vault<T> {
    pub fn new(content: T) -> Self {
        Self { content }
    }

    pub fn encrypt(&self, password: &str) -> EncryptedVault {
        let key = VaultKey::new(password);
        let nonce = secretbox::gen_nonce();
        let plaintext = serde_json::to_vec(&self).unwrap();
        let mut e = GzEncoder::new(Vec::new(), Compression::best());
        e.write_all(&plaintext).unwrap();
        let compressed: Vec<u8> = e.finish().unwrap();
        let data = secretbox::seal(&compressed, &nonce, &key.key);
        EncryptedVault {
            magic: MAGIC,
            version: VERSION,
            nonce,
            data,
            salt: key.salt,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct EncryptedVault {
    /// Magic byte 0xd7
    magic: u8,
    /// Version, enough space for feature bits if needed in the future
    version: u32,
    /// Salt for the password
    salt: pwhash::Salt,
    /// Salt for libsodium
    nonce: secretbox::Nonce,
    /// The actual data as bytes
    data: Vec<u8>,
}
impl EncryptedVault {
    #[must_use]
    pub fn decrypt<T: Content>(self, password: &str) -> Option<Vault<T>> {
        let key = VaultKey::reconstruct(password, self.salt);
        let compressed = secretbox::open(&self.data, &self.nonce, &key.key).ok()?;
        let mut gz = GzDecoder::new(compressed.as_slice());
        let mut plaintext: Vec<u8> = Vec::new();
        gz.read_to_end(&mut plaintext).expect("Decompression failed");
        let vault: Vault<T> = serde_json::from_slice(plaintext.as_slice()).expect("Invalid JSON");
        Some(vault)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    /// Only errors when decoding fails
    #[must_use]
    pub fn from_bytes(data: &[u8]) -> Result<Self, ()> {
        let data: Self = bincode::deserialize(data).map_err(|_| ())?;
        assert!(data.magic == MAGIC, "Invalid magic byte");
        assert!(data.version == VERSION, "Unsupported version");
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::{EncryptedVault, Vault};

    #[test]
    fn encrypt_decrypt() {
        rust_sodium::init().expect("Sodium init failed");

        let password = "TestPass";
        let v = Vault::new(1337u32);
        let ec = v.encrypt(password);
        let bytes = ec.clone().to_bytes();
        let ec2 = EncryptedVault::from_bytes(&bytes).expect("Decode");
        assert_eq!(ec, ec2);
        let v2 = ec2.decrypt(password).expect("Decryption failed");

        assert_eq!(v, v2);
    }

    #[test]
    fn encrypt_decrypt_wrongpass() {
        rust_sodium::init().expect("Sodium init failed");

        let v = Vault::new(1337u32);
        let ec = v.encrypt("TestPass");
        let bytes = ec.clone().to_bytes();
        let ec2 = EncryptedVault::from_bytes(&bytes).expect("Decode");
        assert_eq!(ec, ec2);
        assert!(ec2.decrypt::<u32>("WrongPass") == None);
    }
}
