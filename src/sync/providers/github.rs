//! Uses a private GitHub repository to syncronize passwords.
//! An empty file called VPassFile is used to mark this as a vpass repository

use super::super::{Error, SyncProvider, SyncResult, UpdateKey};
use crate::VResult;

use base64;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use log::debug;

const API_URL: &str = "https://api.github.com";

type ConfigIntermediate = (Vec<u8>, Vec<u8>, [u8; 20], bool);

fn sizeopt_string(s: &str) -> Vec<u8> {
    let len = s.len();
    let mut buf: Vec<u8> = Vec::new();
    if len < std::u8::MAX as usize {
        buf.push(len as u8)
    } else {
        assert!(len <= std::u32::MAX as usize);
        buf.extend(&(len as u32).to_le_bytes());
    }
    buf.extend(s.bytes());
    buf
}

// (String, used_bytes)
fn read_sizeopt_string(s: &[u8]) -> (String, usize) {
    let (len, index) = if s[0] == std::u8::MAX {
        (s[0] as usize, 1)
    } else {
        (u32::from_le_bytes([s[1], s[2], s[3], s[4]]) as usize, 5)
    };

    (
        String::from_utf8(s[index..index + len].to_vec()).unwrap(),
        index + len,
    )
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    username: String,
    access_token: String,
    repo_name: String,
    allow_public_repo: bool,
}
impl Config {
    fn compress(&self) -> Vec<u8> {
        let mut access_token_bytes = [0u8; 20];
        assert_eq!(self.access_token.len(), 40);
        for (i, c) in self.access_token.as_bytes().chunks_exact(2).enumerate() {
            let a = [c[0], c[1]];
            let hex = std::str::from_utf8(&a).unwrap();
            access_token_bytes[i] = u8::from_str_radix(hex, 16).unwrap();
        }

        let mut result: Vec<u8> = Vec::new();
        result.extend(sizeopt_string(&self.username));
        result.extend(sizeopt_string(&self.repo_name));
        result.extend(&access_token_bytes);
        result.push(self.allow_public_repo as u8);
        result
    }

    fn decompress(data: &[u8]) -> VResult<Self> {
        let mut index: usize = 0;
        let (username, l) = read_sizeopt_string(&data[index..]);
        index += l;
        let (repo_name, l) = read_sizeopt_string(&data[index..]);
        index += l;
        let access_token = data[index..index + 20]
            .iter()
            .map(|b| format!("{:x}", b))
            .collect::<Vec<_>>()
            .join("");
        assert!(data[index + 20] <= 1);
        let allow_public_repo = data[index + 20] == 0;

        Ok(Config {
            username,
            repo_name,
            access_token,
            allow_public_repo,
        })
    }
}
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "GitHub_Config({}/{})", self.username, self.repo_name)
    }
}

pub struct GitHub {
    client: reqwest::Client,
    config: Config,
}
impl SyncProvider for GitHub {
    fn interactive_setup() -> VResult<Value> {
        use crate::cli::interactive::*;
        // TODO: "POST" "/authorizations"
        // https://developer.github.com/v3/oauth_authorizations/#create-a-new-authorization
        // TODO: at least provide a link to "https://github.com/settings/tokens/new"
        // TODO: Create VPassFile
        panic!("TODO")
    }

    /// Custom compression for configuration.
    /// Provider should overwrite this to get smaller transfer strings.
    fn configuration_compress(item: &Value) -> Vec<u8>
    where Self: Sized {
        let c: Config = serde_json::from_value(item.clone()).unwrap();
        c.compress()
    }

    /// Custom decompression for configuration.
    /// Provider should overwrite this to get smaller transfer strings.
    fn configuration_decompress(data: &[u8]) -> VResult<Value>
    where Self: Sized {
        let c: Config = Config::decompress(&data)?;
        Ok(serde_json::to_value(&c)?)
    }

    fn load(value: &Value) -> Self
    where Self: Sized {
        GitHub {
            config: serde_json::from_value(value.clone()).expect("Invalid config for GitHub integration"),
            client: reqwest::Client::new(),
        }
    }

    fn ping(&mut self) -> SyncResult<()> {
        debug!("Ping (access check)");
        let mut res = self
            .client
            .get(&format!(
                "{}/repos/{}/{}",
                API_URL, self.config.username, self.config.repo_name
            ))
            .header("Authorization", format!("token {}", self.config.access_token))
            .send()?;

        if res.status().is_success() {
            debug!("Ping ok");
            Ok(())
        } else if res.status() == 403 {
            Err(Error::InvalidCredentials(
                res.json::<Value>()?
                    .get("message")
                    .map(|s| {
                        s.as_str()
                            .expect("GitHub API: Message is not a string")
                            .to_owned()
                    })
                    .unwrap_or_else(|| "No message provided".to_owned()),
            ))
        } else {
            Err(Error::Misc(
                res.json::<Value>()?
                    .get("message")
                    .map(|s| {
                        s.as_str()
                            .expect("GitHub API: Message is not a string")
                            .to_owned()
                    })
                    .unwrap_or_else(|| "No message provided".to_owned()),
            ))
        }
    }

    fn test(&mut self) -> SyncResult<()> {
        // TODO: check that the repo is not public, if required
        let mut res = self
            .client
            .get(&format!(
                "{}/repos/{}/{}/contents/VPassFile",
                API_URL, self.config.username, self.config.repo_name
            ))
            .header("Authorization", format!("token {}", self.config.access_token))
            .send()?;

        if res.status().is_success() {
            Ok(())
        } else if res.status() == 404 {
            Err(Error::InvalidRemote)
        } else {
            Err(Error::Misc(
                res.json::<Value>()?
                    .get("message")
                    .map(|s| {
                        s.as_str()
                            .expect("GitHub API: Message is not a string")
                            .to_owned()
                    })
                    .unwrap_or_else(|| "No message provided".to_owned()),
            ))
        }
    }

    fn create(&mut self, key: &str, value: Vec<u8>) -> SyncResult<()> {
        debug!(
            "Create: {}",
            (format!(
                "{}/repos/{}/{}/contents/{}",
                API_URL, self.config.username, self.config.repo_name, key,
            ))
        );
        let mut res = self
            .client
            .put(&format!(
                "{}/repos/{}/{}/contents/{}",
                API_URL, self.config.username, self.config.repo_name, key,
            ))
            .header("Authorization", format!("token {}", self.config.access_token))
            .json(&json!({
                "message": format!("Create {}", key),
                "content": base64::encode(&value),
            }))
            .send()?;

        if res.status().is_success() {
            Ok(())
        } else {
            Err(Error::Misc(
                res.json::<Value>()?
                    .get("message")
                    .map(|s| {
                        s.as_str()
                            .expect("GitHub API: Message is not a string")
                            .to_owned()
                    })
                    .unwrap_or_else(|| "No message provided".to_owned()),
            ))
        }
    }

    fn update(&mut self, key: &str, value: Vec<u8>, update_key: UpdateKey) -> SyncResult<()> {
        debug!(
            "Update: {}",
            format!(
                "{}/repos/{}/{}/contents/{} ? sha={}",
                API_URL,
                self.config.username,
                self.config.repo_name,
                key,
                update_key.to_string()
            ),
        );
        let mut res = self
            .client
            .put(&format!(
                "{}/repos/{}/{}/contents/{}",
                API_URL, self.config.username, self.config.repo_name, key,
            ))
            .header("Authorization", format!("token {}", self.config.access_token))
            .json(&json!({
                "message": format!("Update {}", key),
                "content": base64::encode(&value),
                "sha": update_key.to_string(),
            }))
            .send()?;

        if res.status().is_success() {
            Ok(())
        } else {
            Err(Error::Misc(
                res.json::<Value>()?
                    .get("message")
                    .map(|s| {
                        s.as_str()
                            .expect("GitHub API: Message is not a string")
                            .to_owned()
                    })
                    .unwrap_or_else(|| "No message provided".to_owned()),
            ))
        }
    }

    fn read(&mut self, key: &str) -> SyncResult<(Vec<u8>, UpdateKey)> {
        debug!(
            "Read: {}",
            format!(
                "{}/repos/{}/{}/contents/{}",
                API_URL, self.config.username, self.config.repo_name, key,
            )
        );
        let mut res = self
            .client
            .get(&format!(
                "{}/repos/{}/{}/contents/{}",
                API_URL, self.config.username, self.config.repo_name, key,
            ))
            .header("Authorization", format!("token {}", self.config.access_token))
            .header("Accept", "application/vnd.github.VERSION.raw")
            .send()?;

        if res.status().is_success() {
            let mut buf: Vec<u8> = Vec::new();
            res.copy_to(&mut buf)?;
            // Etag header contains quoted sha1
            let quoted_sha = res
                .headers()
                .get("etag")
                .expect("GitHub API: etag sha missing")
                .to_str()
                .unwrap();
            Ok((
                buf,
                UpdateKey::from_string(quoted_sha[1..quoted_sha.len() - 1].to_owned()),
            ))
        } else if res.status() == 404 {
            Err(Error::NoSuchKey(key.to_owned()))
        } else {
            Err(Error::Misc(
                res.json::<Value>()?
                    .get("message")
                    .map(|s| {
                        s.as_str()
                            .expect("GitHub API: Message is not a string")
                            .to_owned()
                    })
                    .unwrap_or_else(|| "No message provided".to_owned()),
            ))
        }
    }

    fn delete(&mut self, _key: &str) -> SyncResult<()> {
        unimplemented!()

        // "DELETE" "/repos/:owner/:repo/contents/:path"
    }
}
