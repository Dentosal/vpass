use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug, Clone, PartialEq)]
#[structopt(rename_all = "kebab-case")]
pub struct OptRoot {
    /// Quiet mode: only print errors and prompts
    #[structopt(short, long, group = "loudness")]
    pub quiet: bool,

    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[structopt(short, long, group = "loudness", parse(from_occurrences))]
    pub verbose: u8,

    /// Overrides vault directory path.
    /// Config file is only looked here as well, if not specified separately.
    #[structopt(short = "d", long, env = "VPASS_VAULT_DIR")]
    pub vault_dir: Option<PathBuf>,

    /// Overrides config file path
    #[structopt(short, long, group = "config_xor")]
    pub config: Option<PathBuf>,

    /// Disables looking for config file, uses defaults instead
    #[structopt(long, group = "config_xor")]
    pub disable_config: bool,

    /// Select vault by name
    #[structopt(short = "n", long = "vault", group = "vault")]
    pub vault_name: Option<String>,

    /// Select vault by file path
    #[structopt(short = "f", long = "file", group = "vault")]
    pub vault_file: Option<PathBuf>,

    /// Vault password, takes password as argument instead of prompt
    #[structopt(short, long)]
    pub password: Option<String>,

    /// Subcommand
    #[structopt(subcommand)]
    pub subcommand: SubCommand,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
#[structopt(rename_all = "kebab-case")]
pub enum SubCommand {
    /// Initialize: create necessary directories and config file
    Init,

    /// Manage vaults
    Vault(OptVault),

    /// Add new password entry
    Add(OptAdd),

    /// Update existing entry
    Edit(OptEdit),

    /// Rename entry
    Rename(OptRename),

    /// Remove entry
    Remove(OptRemove),

    /// List entries
    List(OptList),

    /// Display contents of an entry
    Show(OptShow),

    /// Copy password of an entry
    Copy(OptCopy),

    /// Edit synchronization settings of a vault
    Sync(OptSync),

    /// Create or edit config.
    /// By default, creates configuration file if it doesn't exist.
    Config(OptConfig),
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
pub struct OptVault {
    /// Subcommand
    #[structopt(subcommand)]
    pub subcommand: VaultSubCommand,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
#[structopt(rename_all = "kebab-case")]
pub enum VaultSubCommand {
    /// Create a new vault
    Create(OptVaultCreate),
    /// Import a vault from synchronization json
    Import(OptVaultImport),
    /// Rename a vault.
    Rename(OptVaultRename),
    /// Delete a vault locally. Doesn't delete the remote copy.
    Delete(OptVaultDelete),
    /// Copy a vault, disassociating the new copy from remote copy
    Copy(OptVaultCopy),
    /// Change vault password.
    /// This is always synchronized.
    ChangePassword(OptVaultChangePassword),
    /// List vaults
    List(OptVaultList),
    /// Show vault metadata
    Show(OptVaultShow),
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
pub struct OptVaultCreate {
    pub name: String,

    /// Give password as argument instead of prompt
    #[structopt(short, long)]
    pub password: Option<String>,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
#[structopt(rename_all = "kebab-case")]
pub struct OptVaultImport {
    /// Name must match the name on remote.
    /// This restriction might be raised in the future.
    pub name: String,

    /// Use `sync export` to export vaults
    pub import_string: String,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
pub struct OptVaultRename {
    pub old_name: String,
    pub new_name: String,

    /// Keep old version on remote
    #[structopt(long)]
    pub remote_keep_old: bool,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
pub struct OptVaultDelete {
    pub name: String,

    /// Do not prompt for vault password to confirm
    #[structopt(long, group = "exclusive")]
    pub force: bool,

    /// Delete the remote copy too
    #[structopt(long, group = "exclusive")]
    pub remote: bool,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
#[structopt(rename_all = "kebab-case")]
pub struct OptVaultCopy {
    pub old_name: String,
    pub new_name: String,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
#[structopt(rename_all = "kebab-case")]
pub struct OptVaultChangePassword {
    pub name: String,

    /// Give password as argument instead of prompt
    #[structopt(short, long)]
    pub password: Option<String>,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
pub struct OptVaultList {
    /// Output as json
    #[structopt(short, long)]
    pub json: bool,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
pub struct OptVaultShow {
    pub name: String,

    /// Output as json
    #[structopt(short, long)]
    pub json: bool,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
pub struct OptAdd {
    /// Name of the entry
    pub name: String,

    /// One-word tags
    #[structopt(name = "tag", short, long = "tag")]
    pub tags: Vec<String>,

    /// Free-form notes
    #[structopt(name = "note", short, long = "note")]
    pub notes: Vec<String>,

    /// Give password as argument instead of prompt
    #[structopt(short, long, group = "password_exclusive")]
    pub password: Option<String>,

    /// Skip password
    #[structopt(short, long, group = "password_exclusive")]
    pub skip_password: bool,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
pub struct OptEdit {
    /// Name of the entry
    pub name: String,

    /// Name of the entry
    #[structopt(name = "tag", short, long = "tag")]
    pub tags: Vec<String>,

    /// Remove tag (untag)
    #[structopt(name = "remove-tag", short = "u", long = "remove-tag")]
    pub remove_tags: Vec<String>,

    /// Add free-from note to the entry
    #[structopt(short, long = "note")]
    pub notes: Vec<String>,

    /// Remove note by index (zero-indexed)
    #[structopt(name = "remove-note", long = "remove-note")]
    pub remove_notes: Vec<usize>,

    /// Change password, takes password as argument instead of prompt
    #[structopt(short, long, group = "password_exclusive")]
    pub password: Option<String>,

    /// Change password, prompts for a new one
    #[structopt(short, long, group = "password_exclusive")]
    pub change_password: bool,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
#[structopt(rename_all = "kebab-case")]
pub struct OptRename {
    /// Current name of the entry
    pub old_name: String,

    /// New name for the entry
    pub new_name: String,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
pub struct OptRemove {
    pub name: String,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
pub struct OptList {
    /// Output as json
    #[structopt(short, long)]
    pub json: bool,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
pub struct OptShow {
    pub name: String,

    /// Display password in plaintext
    #[structopt(short, long)]
    pub password: bool,

    /// Output as JSON
    #[structopt(short, long)]
    pub json: bool,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
pub struct OptCopy {
    pub name: String,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
pub struct OptSync {
    /// Subcommand
    #[structopt(subcommand)]
    pub subcommand: Option<SyncSubCommand>,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
#[structopt(rename_all = "kebab-case")]
pub enum SyncSubCommand {
    /// Set up synchronization for the vault
    Setup(OptSyncSetup),
    /// Export provider settings as a transfer string.
    /// Use `vault import` to download this vault on another device.
    /// If enabling sync when the same vault has manually been copied to another device,
    /// or changing to another synchronization service, use  `sync setup --import`.
    /// Includes per-provider data, most likely containing access tokens etc.
    Export,
    /// Remove synchronization from the vault
    Detach,
    /// Delete remote vault
    Delete,
    /// Overwrite remote changes, "force push"
    Overwrite,
    /// Show synchronization target.
    Show(OptSyncShow),
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
pub struct OptSyncSetup {
    /// Give options an import string from `sync export`
    #[structopt(short, long, group = "exclusive")]
    pub import: Option<String>,

    /// Give options as json blob
    #[structopt(short, long, group = "exclusive")]
    pub json: Option<String>,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
pub struct OptSyncShow {
    /// Print as JSON. This includes private access tokens etc. from integration config.
    #[structopt(short, long)]
    pub json: bool,
}

#[derive(StructOpt, Debug, Clone, PartialEq)]
pub struct OptConfig {
    /// Print config as JSON
    #[structopt(short, long, group = "exclusive")]
    pub json: bool,

    /// Override the config file with default config
    #[structopt(long, group = "exclusive")]
    pub clear: bool,
}
