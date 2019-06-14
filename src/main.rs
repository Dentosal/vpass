// TODO: proper error handling

use vpass::{self, cli::*};

use serde_json::json;
use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;

#[must_use]
fn prompt_password(prompt: &str) -> Option<String> {
    let pass = rpassword::read_password_from_tty(Some(prompt)).ok()?;
    println!();
    Some(pass)
}

fn list_vaults(args: &opt::OptRoot) -> Vec<String> {
    fs::read_dir(paths::data_dir(args))
        .expect("Vault dir missing")
        .filter_map(|d| {
            let p = d.unwrap().path();
            if p.is_file()
                && p.extension()
                    .map(|s| s.to_str() == Some("vpass_vault"))
                    .unwrap_or(false)
            {
                Some(p.file_stem().unwrap().to_str().unwrap().to_owned())
            } else {
                None
            }
        })
        .collect()
}

fn get_vault_path(args: &opt::OptRoot) -> Option<PathBuf> {
    if let Some(p) = args.vault_file.clone() {
        assert!(p.is_file(), "Unable to find vault file");
        Some(p)
    } else {
        let vaults = list_vaults(args);
        if let Some(name) = args.vault_name.clone() {
            assert!(vaults.contains(&name), "No such vault");
            Some(paths::data_dir(args).join(format!("{}.vpass_vault", name)))
        } else if let Some(name) = cfg::read(args).default_vault.clone() {
            assert!(vaults.contains(&name), "Default vault missing");
            Some(paths::data_dir(args).join(format!("{}.vpass_vault", name)))
        } else {
            None
        }
    }
}

fn main() {
    use opt::*;

    rust_sodium::init().expect("Sodium init failed");

    let args = OptRoot::from_args();

    macro_rules! prompt_vault_password {
        () => {
            args.password
                .clone()
                .unwrap_or_else(|| prompt_password("Password [vault]:").expect("Unable to read password"))
        };
    }

    if args.subcommand != SubCommand::Init {
        assert!(paths::data_dir(&args).is_dir(), "Unitialized");
        assert!(paths::config_file(&args).exists(), "Unitialized");
    }

    match args.subcommand {
        SubCommand::Init => {
            fs::create_dir_all(&paths::data_dir(&args)).expect("Unable to create data dir");
            cfg::write(&args, cfg::Config::default());
            println!("Initialization complete");
        },
        SubCommand::Vault(ref sc) => match sc.subcommand {
            VaultSubCommand::Create(ref c) => {
                validate::vault_name(&c.name).expect("Invalid vault name");
                let pw = prompt_vault_password!();
                let p = paths::data_dir(&args).join(format!("{}.vpass_vault", c.name));
                vpass::create_vault(&p, &pw).expect("Unable to create vault");
            },
            VaultSubCommand::Rename(ref c) => {
                validate::vault_name(&c.new_name).expect("Invalid vault name");

                let vaults = list_vaults(&args);
                assert!(vaults.contains(&c.old_name), "Source vault not found");
                assert!(!vaults.contains(&c.new_name), "Target vault already exists");
                fs::rename(
                    &paths::data_dir(&args).join(&format!("{}.vpass_vault", c.old_name)),
                    &paths::data_dir(&args).join(&format!("{}.vpass_vault", c.new_name)),
                )
                .unwrap();
            },
            VaultSubCommand::Delete(ref c) => {
                let vaults = list_vaults(&args);
                assert!(vaults.contains(&c.name), "Vault not found");
                let p = paths::data_dir(&args).join(&format!("{}.vpass_vault", c.name));
                if !c.force {
                    println!("Confirm vault deletion:");
                    let pw = prompt_vault_password!();
                    vpass::read(&p, &pw).expect("Invalid password, abort");
                }
                fs::remove_file(&p).unwrap();
            },
            VaultSubCommand::ChangePassword(ref c) => {
                let vaults = list_vaults(&args);
                assert!(vaults.contains(&c.name), "Vault not found");
                let p = paths::data_dir(&args).join(&format!("{}.vpass_vault", c.name));
                let pw = prompt_vault_password!();
                let book = vpass::read(&p, &pw).expect("Invalid password, abort");
                let new_pw = c.password.clone().unwrap_or_else(|| {
                    prompt_password("New password [vault]:").expect("Unable to read password")
                });
                vpass::write(&p, &new_pw, book).expect("Unable to write vault");
            },
            VaultSubCommand::List(ref c) => {
                let vaults = list_vaults(&args);
                println!(
                    "{}",
                    if c.json {
                        serde_json::to_string(&vaults).unwrap()
                    } else {
                        vaults.join("\n")
                    }
                );
            },
            VaultSubCommand::Show(ref c) => {
                let vaults = list_vaults(&args);
                assert!(vaults.contains(&c.name), "Vault not found");
                let p = paths::data_dir(&args).join(&format!("{}.vpass_vault", c.name));
                let pw = prompt_vault_password!();
                vpass::read(&p, &pw).expect("Unable to read vault");
                unimplemented!(); // TODO
            },
        },
        SubCommand::Add(ref c) => {
            let p = get_vault_path(&args).expect("Vault not specified");
            let pw = prompt_vault_password!();

            let mut book = vpass::read(&p, &pw).expect("Unable to read vault");
            let items = book.items();
            assert!(
                items.is_empty() || items.iter().all(|item| item.name != c.name),
                "Item already exists"
            );
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
                    .map(|pass| vpass::Password::new(&pass)),
            });
            vpass::write(&p, &pw, book).expect("Unable to write vault");
        },
        SubCommand::Edit(ref c) => {
            let p = get_vault_path(&args).expect("Vault not specified");
            let pw = prompt_vault_password!();

            let mut book = vpass::read(&p, &pw).expect("Unable to read vault");
            if let Some((id, _)) = book.id_items().iter().find(|(_, item)| item.name == c.name) {
                book.modify(*id, |item| {
                    if let Some(ref new_pw) = c.password {
                        item.password = Some(vpass::Password::new(new_pw));
                    } else if c.change_password {
                        item.password = Some(vpass::Password::new(
                            &prompt_password("New password:").expect("Unable to read password"),
                        ));
                    }

                    let indices = c.remove_notes.clone();
                    assert!(indices
                        .iter()
                        .max()
                        .map(|m| m < &item.notes.len())
                        .unwrap_or(true));
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
                })
                .unwrap();

                vpass::write(&p, &pw, book).expect("Unable to write vault");
            } else {
                panic!("Item not found");
            }
        },
        SubCommand::Rename(ref c) => {
            let p = get_vault_path(&args).expect("Vault not specified");
            let pw = prompt_vault_password!();

            let mut book = vpass::read(&p, &pw).expect("Unable to read vault");
            let items = book.items();
            assert!(
                items.is_empty() || items.iter().all(|item| item.name != c.new_name),
                "Item already exists"
            );
            if let Some((id, _)) = book.id_items().iter().find(|(_, item)| item.name == c.old_name) {
                book.modify(*id, |item| {
                    item.name = c.new_name.clone();
                })
                .unwrap();

                vpass::write(&p, &pw, book).expect("Unable to write vault");
            } else {
                panic!("Item not found");
            }
        },
        SubCommand::Remove(ref c) => {
            let p = get_vault_path(&args).expect("Vault not specified");
            let pw = prompt_vault_password!();

            let mut book = vpass::read(&p, &pw).expect("Unable to read vault");
            if let Some((id, _)) = book.id_items().iter().find(|(_, item)| item.name == c.name) {
                book.remove(*id);
                vpass::write(&p, &pw, book).expect("Unable to write vault");
            } else {
                panic!("Item not found");
            }
        },
        SubCommand::List(ref c) => {
            let p = get_vault_path(&args).expect("Vault not specified");
            let pw = prompt_vault_password!();
            let book = vpass::read(&p, &pw).expect("Unable to read vault");

            let items: Vec<String> = book.items().iter().map(|item| item.name.clone()).collect();
            println!(
                "{}",
                if c.json {
                    serde_json::to_string(&items).unwrap()
                } else {
                    items.join("\n")
                }
            );
        },
        SubCommand::Show(ref c) => {
            let p = get_vault_path(&args).expect("Vault not specified");
            let pw = prompt_vault_password!();
            let book = vpass::read(&p, &pw).expect("Unable to read vault");
            if let Some((item, meta)) = book.items_metadata().iter().find(|(item, _)| item.name == c.name) {
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
                        println!("password: {}", item.password.clone().unwrap().plaintext()); // Special handling for some cases
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
            } else {
                eprintln!("Item {:?} not found", c.name);
            }
        },
        SubCommand::Copy(ref c) => {
            let p = get_vault_path(&args).expect("Vault not specified");
            let pw = prompt_vault_password!();
            let book = vpass::read(&p, &pw).expect("Unable to read vault");
            if let Some(item) = book.items().iter().find(|item| item.name == c.name) {
                if let Some(ref item_pw) = item.password {
                    clipboard::write(&item_pw.plaintext());
                } else {
                    eprintln!("No password set for item");
                }
            } else {
                eprintln!("Item {:?} not found", c.name);
            }
        },
        SubCommand::Config(ref c) => {
            let config = cfg::read(&args);

            if c.json {
                println!("{}", String::from_utf8(config.to_json_bytes()).unwrap());
            } else if c.clear {
                cfg::write(&args, cfg::Config::default());
            } else {
                println!("{}", config.to_json_pretty());
            }
        },
    }
}
