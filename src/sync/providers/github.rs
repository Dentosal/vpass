//! Uses a private GitHub repository to syncronize passwords.
//! An empty file called VPassFile is used to mark this as a vpass repository

use super::super::{Error, SyncProvider, SyncResult, UpdateKey};
use crate::VResult;

use base64;
use reqwest::header::{HeaderMap, AUTHORIZATION};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::convert::AsMut;

use log::debug;

const API_URL: &str = "https://api.github.com";

type ConfigIntermediate = (Vec<u8>, Vec<u8>, [u8; 20], bool);

fn clone_into_array<A, T>(slice: &[T]) -> A
where
    A: Sized + Default + AsMut<[T]>,
    T: Clone,
{
    let mut a = Default::default();
    <A as AsMut<[T]>>::as_mut(&mut a).clone_from_slice(slice);
    a
}

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

/// Returns: (String, used_bytes)
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

fn hex_string(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:x}", b))
        .collect::<Vec<_>>()
        .join("")
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    username: String,
    access_token: String,
    access_token_id: u64,
    repo_name: String,
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
        result.extend(self.access_token_id.to_le_bytes().iter());
        result
    }

    fn decompress(data: &[u8]) -> VResult<Self> {
        let mut index: usize = 0;
        let (username, l) = read_sizeopt_string(&data[index..]);
        index += l;
        let (repo_name, l) = read_sizeopt_string(&data[index..]);
        index += l;
        let access_token = hex_string(&data[index..index + 20]);
        index += 20;
        let access_token_id = u64::from_le_bytes(clone_into_array(&data[index..index + 8]));

        Ok(Config {
            username,
            repo_name,
            access_token,
            access_token_id,
        })
    }
}
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "GitHub_Config({}/{})", self.username, self.repo_name)
    }
}

/// Returns `Ok(Ok(token_id, token))` on success,
/// and `Ok(Err(OTP_METHOD))` if oauth token is required but missing.
fn create_oauth_token(
    username: &str, password: &str, otp_code: Option<String>,
) -> VResult<Result<(u64, String), String>> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

    let client = reqwest::Client::new();
    let mut req = client
        .post(&format!("{}/authorizations", API_URL))
        .header(
            AUTHORIZATION,
            format!("basic {}", base64::encode(&format!("{}:{}", username, password))),
        )
        .json(&serde_json::json!({
            "scopes": ["repo"],
            "note": format!("VPass synchronization token (ts: {})", since_epoch.as_millis())
        }));

    if let Some(otp) = otp_code {
        req = req.header("x-github-otp", otp);
    }

    let mut res = req.send().map_err(Error::Http)?;

    let h = res.headers();
    if res.status() == 401 {
        if let Some(otp_header) = h.get("x-github-otp") {
            Ok(Err(otp_header.to_str().unwrap().to_owned()))
        } else {
            Err(Error::InvalidCredentials(
                res.json::<Value>()
                    .map_err(Error::Http)?
                    .get("message")
                    .map(|s| {
                        s.as_str()
                            .expect("GitHub API: Message is not a string")
                            .to_owned()
                    })
                    .unwrap_or_else(|| "No message provided".to_owned()),
            )
            .into())
        }
    } else {
        let j = res.json::<Value>().map_err(Error::Http)?;
        let id = j.get("id").expect("Missing key: id");
        let token = j.get("token").expect("Missing key: token");
        Ok(Ok((
            id.as_u64().expect("token_id must be an u64"),
            token.as_str().expect("token must be a string").to_owned(),
        )))
    }
}

/// https://developer.github.com/v3/oauth_authorizations/#create-a-new-authorization
fn create_oauth_token_interactive(username: &str) -> VResult<(u64, String)> {
    use crate::cli::interactive::*;
    let password = prompt_password("Password")?;

    match create_oauth_token(username, &password, None)? {
        Ok(token) => Ok(token),
        Err(otp_header) => {
            let otp_code = match otp_header.as_str() {
                "required; app" => prompt_string("2FA code from app")?,
                "required; SMS" => prompt_string("2FA code from SMS")?,
                other => panic!("Unknown GitHub OTP header: {:?}", other),
            };
            Ok(create_oauth_token(username, &password, Some(otp_code))?.unwrap())
        },
    }
}

fn wrap_response<F, R>(mut res: Response, mut f: F) -> SyncResult<R>
where F: FnMut(Response) -> SyncResult<R> {
    if res.status().is_success() {
        f(res)
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
        Err(Error::HttpStatus(res.status().as_u16(), Some(res.json()?)))
    }
}

fn wrap_response_json(res: Response) -> SyncResult<(Value, HeaderMap)> {
    wrap_response(res, |mut r| {
        let h = r.headers().clone();
        let j = r.json()?;
        Ok((j, h))
    })
}

fn wrap_response_raw(res: Response) -> SyncResult<(Vec<u8>, HeaderMap)> {
    wrap_response(res, |mut r| {
        let h = r.headers().clone();
        let mut buf: Vec<u8> = Vec::new();
        r.copy_to(&mut buf)?;
        Ok((buf, h))
    })
}

pub struct GitHub {
    client: reqwest::Client,
    config: Config,
}
impl GitHub {
    fn api_get_raw(&self, path: &str) -> SyncResult<(Vec<u8>, HeaderMap)> {
        debug!("Get raw: {}", path);
        wrap_response_raw(
            self.client
                .get(&format!("{}/{}", API_URL, path))
                .header(AUTHORIZATION, format!("token {}", self.config.access_token))
                .header("Accept", "application/vnd.github.VERSION.raw")
                .send()?,
        )
    }

    fn api_get(&self, path: &str) -> SyncResult<(Value, HeaderMap)> {
        debug!("Get: {}", path);
        wrap_response_json(
            self.client
                .get(&format!("{}/{}", API_URL, path))
                .header(AUTHORIZATION, format!("token {}", self.config.access_token))
                .send()?,
        )
    }

    fn api_put(&self, path: &str, value: &Value) -> SyncResult<(Value, HeaderMap)> {
        let s = serde_json::to_string(value).unwrap();
        debug!(
            "Put: {} {}",
            path,
            if s.len() > 40 {
                format!("{}...", s.chars().take(10).collect::<String>())
            } else {
                s.to_owned()
            }
        );
        wrap_response_json(
            self.client
                .put(&format!("{}/{}", API_URL, path))
                .header(AUTHORIZATION, format!("token {}", self.config.access_token))
                .json(value)
                .send()?,
        )
    }

    fn api_delete(&self, path: &str, value: &Value) -> SyncResult<(Value, HeaderMap)> {
        let s = serde_json::to_string(value).unwrap();
        debug!(
            "Delete: {} {}",
            path,
            if s.len() > 40 {
                format!("{}...", s.chars().take(10).collect::<String>())
            } else {
                s.to_owned()
            }
        );
        wrap_response_json(
            self.client
                .delete(&format!("{}/{}", API_URL, path))
                .header(AUTHORIZATION, format!("token {}", self.config.access_token))
                .json(value)
                .send()?,
        )
    }
}
impl SyncProvider for GitHub {
    fn interactive_setup() -> VResult<Value> {
        use crate::cli::interactive::*;

        let username = prompt_string("Username")?;

        let (access_token_id, access_token) = create_oauth_token_interactive(&username)?;

        println!("Login successful");

        let mut self_ = loop {
            let mut self_ = Self {
                client: reqwest::Client::new(),
                config: Config {
                    username: username.clone(),
                    access_token_id,
                    access_token: access_token.clone(),
                    repo_name: prompt_string("Repository name")?,
                },
            };

            match self_.api_get(&format!(
                "repos/{}/{}",
                self_.config.username, self_.config.repo_name
            )) {
                Ok((j, _)) => {
                    if !j
                        .get("private")
                        .expect("GitHub API: Key missing")
                        .as_bool()
                        .expect("GitHub API: Bool required")
                    {
                        println!("{:?} is not private.", self_.config.repo_name);
                        continue;
                    }

                    if self_.exists("VPassFile")? {
                        println!("Using existing vpass repository");
                        break self_;
                    } else {
                        println!("Repository {:?} already exists", self_.config.repo_name);
                        if prompt_boolean("Use existing repo?")? {
                            let (j, _) = self_.api_get(&format!(
                                "repos/{}/{}/contents/",
                                self_.config.username, self_.config.repo_name,
                            ))?;
                            if j.as_array().expect("GitHub API: Array required").is_empty() {
                                self_.create("VPassFile", vec![])?;
                            } else {
                                println!("Non-empty repository cannot be used automatically.");
                                println!(
                                    "To bypass this restriction, create an empty file called VPassFile in the repository root."
                                );
                            }
                        }
                    }
                },
                Err(Error::HttpStatus(404, _)) => {
                    println!("Repository {:?} doesn't exist", self_.config.repo_name);
                    println!("New repo creation is not yet supported by the interactive setup");
                    println!("Please create the repository by hand: https://github.com/new");
                    println!("(remember to specify a private repository)");
                    println!("and then enter it's name here to continue");
                    // TODO: Create the repository automatically
                    // if prompt_boolean("Create a new repo?")? {...}
                },
                Err(other) => return Err(other.into()),
            }
        };

        self_.test()?;

        println!("Repository initialized");

        Ok(serde_json::to_value(self_.config).unwrap())
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
        self.api_get(&format!(
            "repos/{}/{}",
            self.config.username, self.config.repo_name
        ))?;
        debug!("Ping ok");
        Ok(())
    }

    fn test(&mut self) -> SyncResult<()> {
        let (j, _) = self.api_get(&format!(
            "repos/{}/{}",
            self.config.username, self.config.repo_name
        ))?;

        if !j
            .get("private")
            .expect("GitHub API: Key missing")
            .as_bool()
            .expect("GitHub API: Bool required")
        {
            return Err(Error::InsufficientSecurity(
                "Repository must be private".to_owned(),
            ));
        }

        // Check that the repo is not public
        // Check that this is a vpass repository
        self.api_get(&format!(
            "repos/{}/{}/contents/VPassFile",
            self.config.username, self.config.repo_name
        ))?;
        Ok(())
    }

    fn create(&mut self, key: &str, value: Vec<u8>) -> SyncResult<()> {
        self.api_put(
            &format!(
                "repos/{}/{}/contents/{}",
                self.config.username, self.config.repo_name, key,
            ),
            &json!({
                "message": format!("Create {}", key),
                "content": base64::encode(&value),
            }),
        )?;
        Ok(())
    }

    fn update(&mut self, key: &str, value: Vec<u8>, update_key: UpdateKey) -> SyncResult<()> {
        self.api_put(
            &format!(
                "repos/{}/{}/contents/{}",
                self.config.username, self.config.repo_name, key,
            ),
            &json!({
                "message": format!("Create {}", key),
                "content": base64::encode(&value),
                "sha": update_key.to_string(),
            }),
        )?;
        Ok(())
    }

    fn read(&mut self, key: &str) -> SyncResult<(Vec<u8>, UpdateKey)> {
        match self.api_get_raw(&format!(
            "repos/{}/{}/contents/{}",
            self.config.username, self.config.repo_name, key,
        )) {
            Ok((raw, headers)) => {
                // Etag header contains quoted sha1
                let quoted_sha = headers
                    .get("etag")
                    .expect("GitHub API: etag sha missing")
                    .to_str()
                    .unwrap();

                Ok((
                    raw,
                    UpdateKey::from_string(quoted_sha[1..quoted_sha.len() - 1].to_owned()),
                ))
            },
            Err(Error::HttpStatus(404, _)) => Err(Error::NoSuchKey(key.to_owned())),
            Err(other) => Err(other),
        }
    }

    fn delete(&mut self, key: &str, update_key: UpdateKey) -> SyncResult<()> {
        self.api_delete(
            &format!(
                "repos/{}/{}/contents/{}",
                self.config.username, self.config.repo_name, key,
            ),
            &json!({
                "message": format!("Delete {}", key),
                "sha": update_key.to_string(),
            }),
        )?;
        Ok(())
    }
}
