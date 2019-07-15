#![deny(unused_must_use)]

use vpass::{self, cli::*, Password};

use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;

#[must_use]
fn prompt_password(prompt: &str) -> VResult<String> {
    loop {
        let pass = rpassword::read_password_from_tty(Some(prompt))?;
        println!();
        if pass != "" {
            return Ok(pass);
        }
    }
}

struct Vaults(HashSet<String>);
impl Vaults {
    fn new(args: &opt::OptRoot) -> VResult<Self> {
        let dir = paths::data_dir(args)?;
        Ok(Self(
            fs::read_dir(&dir)
                .map_err(|_| Error::VaultDirNotFound(dir))?
                .filter_map(|d| {
                    let p = d.unwrap().path();
                    if p.is_file()
                        && p.extension()
                            .map(|s| s.to_str() == Some("vpass_vault"))
                            .unwrap_or(false)
                    {
                        Some(p.file_stem()?.to_str()?.to_owned())
                    } else {
                        None
                    }
                })
                .collect(),
        ))
    }

    fn to_vec(&self) -> Vec<String> {
        let mut v: Vec<String> = self.0.iter().cloned().collect();
        v.sort();
        v
    }

    fn contains(&self, name: &str) -> bool {
        self.0.contains(name)
    }

    fn verify_exists(&self, name: &str) -> VResult<()> {
        if self.contains(name) {
            Ok(())
        } else {
            Err(Error::VaultNotFound(name.to_owned()))
        }
    }

    fn verify_not_exists(&self, name: &str) -> VResult<()> {
        if self.contains(name) {
            Err(Error::VaultAlreadyExists(name.to_owned()))
        } else {
            Ok(())
        }
    }
}

fn get_vault_path(args: &opt::OptRoot) -> VResult<PathBuf> {
    if let Some(p) = args.vault_file.clone() {
        if p.is_file() {
            Ok(p)
        } else {
            Err(Error::FileRequired(p))
        }
    } else {
        let vaults = Vaults::new(args)?;
        if let Some(name) = args.vault_name.clone() {
            vaults.verify_exists(&name)?;
            vault_path(args, &name)
        } else if let Some(name) = cfg::read(args)?.default_vault.clone() {
            vaults.verify_exists(&name)?;
            vault_path(args, &name)
        } else {
            Err(Error::VaultNotSpecified)
        }
    }
}

fn vault_filename(name: &str) -> String {
    debug_assert!(!name.ends_with(".vpass_vault")); // Sanity check
    format!("{}.vpass_vault", name)
}

fn vault_path(args: &opt::OptRoot, name: &str) -> VResult<PathBuf> {
    Ok(paths::data_dir(&args)?.join(vault_filename(name)))
}

fn main() -> VResult<()> {
    pretty_env_logger::init();
    rust_sodium::init().expect("Sodium init failed");
    run_command(opt::OptRoot::from_args())
}

fn run_command(args: opt::OptRoot) -> VResult<()> {
    use opt::*;
    use vpass::sync;

    macro_rules! prompt_vault_password {
        () => {
            args.password
                .clone()
                .unwrap_or_else(|| prompt_password("Password [vault]:").expect("Unable to read password"))
        };
    }

    if args.subcommand != SubCommand::Init
        && !(paths::data_dir(&args)?.is_dir() && paths::config_file(&args)?.exists())
    {
        return Err(Error::NotInitialized);
    }

    match args.subcommand {
        SubCommand::Init => {
            fs::create_dir_all(&paths::data_dir(&args)?)?;
            cfg::write(&args, cfg::Config::default())?;
            if !args.quiet {
                println!("Initialization complete");
            }
        },
        SubCommand::Vault(ref sc) => match sc.subcommand {
            VaultSubCommand::Create(ref c) => {
                validate::vault_name(&c.name)?;
                let pw = c.password.clone().unwrap_or_else(|| {
                    prompt_password("New password for vault:").expect("Unable to read password")
                });
                let p = vault_path(&args, &c.name)?;
                vpass::create(&p, &pw)?;
            },
            VaultSubCommand::Import(ref c) => {
                validate::vault_name(&c.name)?;
                let p = vault_path(&args, &c.name)?;
                let pw = prompt_vault_password!();
                let transfer_options = sync::transfer_string::decode(&c.import_string)?;
                // TODO: non-default locations?
                let name = p.file_name().unwrap().to_str().unwrap();
                let book = vpass::sync::download_book(name, transfer_options, &pw)?;
                vpass::write(&p, &pw, book)?;
            },
            VaultSubCommand::Rename(ref c) => {
                validate::vault_name(&c.new_name)?;

                let vaults = Vaults::new(&args)?;
                vaults.verify_exists(&c.old_name)?;
                vaults.verify_not_exists(&c.new_name)?;

                let old_p = vault_path(&args, &c.old_name)?;
                let new_p = vault_path(&args, &c.new_name)?;
                let pw = prompt_vault_password!();
                let book = vpass::read(&old_p, &pw)?;
                vpass::sync::check_rename(&vault_filename(&c.new_name), &book)?;

                // Push the new vault to remote
                vpass::sync::create(&vault_filename(&c.new_name), &book, &pw)?;
                // Rename local vault
                match fs::rename(&old_p, &new_p) {
                    Ok(()) => {
                        // Local file renamed, remove old file from remote
                        if !c.remote_keep_old {
                            vpass::sync::delete(&vault_filename(&c.old_name), &book)?;
                        }
                    },
                    Err(e) => {
                        // Could not rename local file: Roll back remote changes
                        vpass::sync::delete(&vault_filename(&c.new_name), &book)?;
                        return Err(e.into());
                    },
                }
            },
            VaultSubCommand::Delete(ref c) => {
                let vaults = Vaults::new(&args)?;
                vaults.verify_exists(&c.name)?;
                let p = vault_path(&args, &c.name)?;
                if !c.force {
                    println!("Confirm vault deletion:");
                    let pw = prompt_vault_password!();
                    let book = vpass::read(&p, &pw)?;
                    if c.remote {
                        // Delete remote first, as if there are errors,
                        // retry isn't possible withtout a local copy
                        vpass::sync::delete(&vault_filename(&c.name), &book)?;
                    }
                }
                fs::remove_file(&p).unwrap();
            },
            VaultSubCommand::Copy(ref c) => {
                validate::vault_name(&c.new_name)?;

                let vaults = Vaults::new(&args)?;
                vaults.verify_exists(&c.old_name)?;
                vaults.verify_not_exists(&c.new_name)?;

                let old_p = vault_path(&args, &c.old_name)?;
                let new_p = vault_path(&args, &c.new_name)?;

                // Copy the file
                fs::copy(&old_p, &new_p)?;

                // Run detach command on the new file
                let mut args_inner = args.clone();
                args_inner.vault_file = Some(new_p.clone());
                args_inner.subcommand = SubCommand::Sync(OptSync {
                    subcommand: Some(SyncSubCommand::Detach),
                });
                match run_command(args_inner) {
                    Ok(()) => {},
                    Err(e) => {
                        // Could not detach: remove the new copy
                        fs::remove_file(&new_p)?;
                        return Err(e);
                    },
                }
            },
            VaultSubCommand::ChangePassword(ref c) => {
                let vaults = Vaults::new(&args)?;
                vaults.verify_exists(&c.name)?;
                let p = vault_path(&args, &c.name)?;
                let old_pw = prompt_vault_password!();
                let book = vpass::read(&p, &old_pw)?;
                let new_pw = if let Some(ref x) = c.password {
                    x.clone()
                } else {
                    prompt_password("New password [vault]:")?
                };

                // Push the new version to remote
                vpass::sync::vault_overwrite(&vault_filename(&c.name), &book, &new_pw)?;

                // Change local vault password
                match vpass::write(&p, &new_pw, book.clone()) {
                    Ok(()) => {},
                    Err(e) => {
                        // Could not change local file password: Roll back remote changes
                        vpass::sync::vault_overwrite(&vault_filename(&c.name), &book, &old_pw)?;
                        return Err(e);
                    },
                }
            },
            VaultSubCommand::List(ref c) => {
                let vaults = Vaults::new(&args)?;
                println!(
                    "{}",
                    if c.json {
                        serde_json::to_string(&vaults.to_vec()).unwrap()
                    } else {
                        vaults.to_vec().join("\n")
                    }
                );
            },
            VaultSubCommand::Show(ref c) => {
                let vaults = Vaults::new(&args)?;
                vaults.verify_exists(&c.name)?;
                let p = vault_path(&args, &c.name)?;
                let pw = prompt_vault_password!();
                let book = vpass::read(&p, &pw)?;
                if c.json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "file_path": p,
                            "creation_time": book.creation_time(),
                            "item_count": book.item_names().len(),
                            "synchronization_service":
                                if let Some(config) = vpass::sync::config::book_read(&book)? {
                                    serde_json::Value::String(format!("{:?}", config.service))
                                } else {
                                    serde_json::Value::Null
                                },
                        })
                        .to_string()
                    )
                } else {
                    println!("File path: {:?}", p);
                    println!("Creation time: {}", book.creation_time());
                    println!("Item count: {}", book.item_names().len());
                    if let Some(config) = vpass::sync::config::book_read(&book)? {
                        println!("Synchronization: {:?}", config.service);
                    } else {
                        println!("Synchronization not set up");
                    }
                }
            },
        },
        SubCommand::Add(ref c) => {
            let p = get_vault_path(&args)?;
            let pw = prompt_vault_password!();

            validate::item_name(&c.name)?;

            let mut book = vpass::read(&p, &pw)?;
            if book.has_item(&c.name) {
                return Err(Error::ItemAlreadyExists(c.name.clone()));
            }
            book.add(vpass::Item {
                name: c.name.clone(),
                tags: c.tags.iter().cloned().collect(),
                notes: c.notes.clone(),
                password: c
                    .password
                    .clone()
                    .or_else(|| {
                        if c.skip_password {
                            None
                        } else {
                            Some(prompt_password("Password [item]:").expect("Unable to read password"))
                        }
                    })
                    .map(|pass| Password::new(&pass)),
            })?;
            vpass::write(&p, &pw, book)?;
        },
        SubCommand::Edit(ref c) => {
            let p = get_vault_path(&args)?;
            let pw = prompt_vault_password!();

            let mut book = vpass::read(&p, &pw)?;
            book.modify_by_name(&c.name, |item| -> VResult<()> {
                if let Some(ref new_pw) = c.password {
                    item.password = Some(Password::new(new_pw));
                } else if c.change_password {
                    item.password = Some(Password::new(&prompt_password("New password:")?));
                }

                let indices = c.remove_notes.clone();
                assert!(
                    indices
                        .iter()
                        .max()
                        .map(|m| *m < item.notes.len())
                        .unwrap_or(true)
                );
                item.notes = item
                    .notes
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !indices.contains(i))
                    .map(|(_, v)| v)
                    .chain(c.notes.iter())
                    .cloned()
                    .collect();

                item.tags = item
                    .tags
                    .iter()
                    .filter(|tag| !c.remove_tags.contains(tag))
                    .chain(c.tags.iter())
                    .cloned()
                    .collect();
                Ok(())
            })
            .unwrap()?;

            vpass::write(&p, &pw, book)?;
        },
        SubCommand::Rename(ref c) => {
            let p = get_vault_path(&args)?;
            let pw = prompt_vault_password!();

            validate::item_name(&c.new_name)?;

            let mut book = vpass::read(&p, &pw)?;
            book.verify_not_exists(&c.new_name)?;
            book.modify_by_name(&c.old_name, |item| {
                item.name = c.new_name.clone();
            })?;
        },
        SubCommand::Remove(ref c) => {
            let p = get_vault_path(&args)?;
            let pw = prompt_vault_password!();

            let mut book = vpass::read(&p, &pw)?;
            book.remove(&c.name)?;
            vpass::write(&p, &pw, book)?;
        },
        SubCommand::List(ref c) => {
            let p = get_vault_path(&args)?;
            let pw = prompt_vault_password!();
            let book = vpass::read(&p, &pw)?;

            println!(
                "{}",
                if c.json {
                    serde_json::to_string(&book.item_names()).unwrap()
                } else {
                    book.item_names().join("\n")
                }
            );
        },
        SubCommand::Show(ref c) => {
            let p = get_vault_path(&args)?;
            let pw = prompt_vault_password!();
            let book = vpass::read(&p, &pw)?;
            let (item, meta) = book.get_item_and_metadata(&c.name)?;
            if c.json {
                let mut j = serde_json::to_value(&item).unwrap();
                j.as_object_mut().unwrap().insert("meta".to_owned(), json!(meta));
                if !c.password {
                    j.as_object_mut().unwrap().remove("password");
                }
                println!("{}", serde_json::to_string(&j).unwrap());
            } else {
                println!("{}", item.name);
                if item.password.is_none() {
                    println!("password not stored");
                } else if c.password {
                    println!("password: {}", item.password.clone().unwrap().plaintext()); // TODO: Special handling for some cases
                } else {
                    println!("password: ********");
                }
                if !item.tags.is_empty() {
                    let mut tags: Vec<String> = item.tags.iter().cloned().collect();
                    tags.sort();
                    println!("tags: {}", tags.join(", "));
                }
                if !item.notes.is_empty() {
                    println!("notes:");
                    for note in &item.notes {
                        println!("> {}", note);
                    }
                }
                println!("created: {}", meta.created);
                println!("changed: {}", meta.changed);
            }
        },
        SubCommand::Copy(ref c) => {
            let p = get_vault_path(&args)?;
            let pw = prompt_vault_password!();
            let book = vpass::read(&p, &pw)?;
            let item = book.get_item_by_name(&c.name)?;
            if let Some(ref item_pw) = item.password {
                clipboard::write(&item_pw.plaintext());
            } else {
                return Err(Error::ItemNoPasswordSet);
            }
        },
        SubCommand::Sync(ref sc) => match sc.subcommand {
            None => {
                let p = get_vault_path(&args)?;
                let pw = prompt_vault_password!();
                let mut book = vpass::read(&p, &pw)?;
                // TODO: non-default locations?
                let name = p.file_name().unwrap().to_str().unwrap();
                vpass::sync::vault(name, &mut book, &pw)?;
                vpass::write(&p, &pw, book)?;
            },
            Some(SyncSubCommand::Setup(ref c)) => {
                let p = get_vault_path(&args)?;
                let pw = prompt_vault_password!();
                let mut book = vpass::read(&p, &pw)?;
                if let Some(ref import_data) = c.import {
                    vpass::sync::config::book_setup(&mut book, sync::transfer_string::decode(&import_data)?)?;
                    vpass::write(&p, &pw, book)?;
                } else if let Some(ref json_data) = c.json {
                    vpass::sync::config::book_setup(&mut book, serde_json::from_str(json_data)?)?;
                    vpass::write(&p, &pw, book)?;
                } else {
                    // Interactive setup
                    if let Some(config) = vpass::cli::interactive::sync_setup(&book)? {
                        vpass::sync::config::book_setup(&mut book, config)?;
                        vpass::write(&p, &pw, book)?;
                    } else {
                        println!("Cancelled");
                    }
                }
            },
            Some(SyncSubCommand::Export) => {
                let p = get_vault_path(&args)?;
                let pw = prompt_vault_password!();
                let book = vpass::read(&p, &pw)?;
                if let Some(s) = sync::config::book_read(&book)? {
                    println!("{}", sync::transfer_string::encode(&s));
                } else {
                    return Err(vpass::sync::Error::NoRemoteSet.into());
                }
            },
            Some(SyncSubCommand::Detach) => {
                let p = get_vault_path(&args)?;
                let pw = prompt_vault_password!();
                let mut book = vpass::read(&p, &pw)?;
                vpass::sync::config::book_remove(&mut book)?;
                vpass::write(&p, &pw, book)?;
            },
            Some(SyncSubCommand::Overwrite) => {
                let p = get_vault_path(&args)?;
                let pw = prompt_vault_password!();
                let book = vpass::read(&p, &pw)?;
                // TODO: non-default locations?
                let name = p.file_name().unwrap().to_str().unwrap();
                vpass::sync::vault_overwrite(name, &book, &pw)?;
            },
            Some(SyncSubCommand::Show) => {
                let p = get_vault_path(&args)?;
                let pw = prompt_vault_password!();
                let book = vpass::read(&p, &pw)?;
                if let Some(config) = vpass::sync::config::book_read(&book)? {
                    println!("{:?}", config.service);
                } else {
                    println!("Synchronization not set up");
                }
            },
        },
        SubCommand::Config(ref c) => {
            let config = cfg::read(&args)?;

            if c.json {
                println!("{}", String::from_utf8(config.to_json_bytes()).unwrap());
            } else if c.clear {
                cfg::write(&args, cfg::Config::default())?;
            } else {
                println!("{}", config.to_json_pretty());
            }
        },
    }

    Ok(())
}
