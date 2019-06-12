use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug, PartialEq)]
#[structopt(rename_all = "kebab-case")]
pub struct OptRoot {
    /// Quiet mode: don't print anything if not spefically requested
    #[structopt(short, long, group = "loudness")]
    pub silent: bool,

    /// Quiet mode: only print errors and prompts
    #[structopt(short, long, group = "loudness")]
    pub quiet: bool,

    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[structopt(short, long, group = "loudness", parse(from_occurrences))]
    pub verbose: u8,

    /// Overrides vault directory path.
    /// Config file is only looked here as well, if not specified separately.
    #[structopt(short = "d", long)]
    pub vault_dir: Option<PathBuf>,

    /// Overrides config file path
    #[structopt(short, long, group = "config")]
    pub config: Option<PathBuf>,

    /// Disables looking for config file, uses defaults instead
    #[structopt(long, group = "config")]
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

#[derive(StructOpt, Debug, PartialEq)]
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

    /// Create or edit config.
    /// By default, creates configuration file if it doesn't exist.
    Config(OptConfig),
}

#[derive(StructOpt, Debug, PartialEq)]
pub struct OptVault {
    /// Subcommand
    #[structopt(subcommand)]
    pub subcommand: VaultSubCommand,
}

#[derive(StructOpt, Debug, PartialEq)]
#[structopt(rename_all = "kebab-case")]
pub enum VaultSubCommand {
    /// Create a new vault
    Create(OptVaultCreate),
    /// Rename a vault
    Rename(OptVaultRename),
    /// Delete a vault
    Delete(OptVaultDelete),
    /// Change vault password
    ChangePassword(OptVaultChangePassword),
    /// List vaults
    List(OptVaultList),
    /// Show vault metadata
    Show(OptVaultShow),
}

#[derive(StructOpt, Debug, PartialEq)]
pub struct OptVaultCreate {
    pub name: String,
}

#[derive(StructOpt, Debug, PartialEq)]
pub struct OptVaultRename {
    pub old_name: String,
    pub new_name: String,
}

#[derive(StructOpt, Debug, PartialEq)]
pub struct OptVaultDelete {
    #[structopt(name = "name", group = "xor")]
    pub names: Vec<String>,

    #[structopt(short, long, group = "xor")]
    pub all: bool,

    /// Do not prompt for vault password to confirm
    #[structopt(long)]
    pub force: bool,
}

#[derive(StructOpt, Debug, PartialEq)]
pub struct OptVaultChangePassword {
    /// Give password as argument instead of prompt
    #[structopt(short, long, group = "password")]
    pub password: Option<String>,
}

#[derive(StructOpt, Debug, PartialEq)]
pub struct OptVaultList {
    /// Output as json
    #[structopt(short, long)]
    pub json: bool,
}

#[derive(StructOpt, Debug, PartialEq)]
pub struct OptVaultShow {
    pub name: String,

    /// Output as json
    #[structopt(short, long)]
    pub json: bool,
}

#[derive(StructOpt, Debug, PartialEq)]
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
    #[structopt(short, long, group = "password")]
    pub password: Option<String>,

    /// Skip password
    #[structopt(short, long, group = "password")]
    pub skip_password: bool,
}

#[derive(StructOpt, Debug, PartialEq)]
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

    /// Change password, prompts for a new one
    #[structopt(short, long, group = "password")]
    pub password: Option<String>,

    /// Change password, takes password as argument instead of prompt
    #[structopt(short, long, group = "password")]
    pub change_passsword: bool,
}

#[derive(StructOpt, Debug, PartialEq)]
#[structopt(rename_all = "kebab-case")]
pub struct OptRename {
    /// Current name of the entry
    pub old_name: String,

    /// New name for the entry
    pub new_name: String,
}

#[derive(StructOpt, Debug, PartialEq)]
pub struct OptRemove {
    pub name: String,
}

#[derive(StructOpt, Debug, PartialEq)]
pub struct OptList {
    /// Output as json
    #[structopt(short, long)]
    pub json: bool,
}

#[derive(StructOpt, Debug, PartialEq)]
pub struct OptShow {
    pub name: String,

    /// Display password in plaintext
    #[structopt(short, long)]
    pub password: bool,

    /// Output as JSON
    #[structopt(short, long)]
    pub json: bool,
}

#[derive(StructOpt, Debug, PartialEq)]
pub struct OptCopy {
    pub name: String,
}

#[derive(StructOpt, Debug, PartialEq)]
pub struct OptConfig {
    /// Print config
    #[structopt(short, long, group = "xor")]
    pub print: bool,

    /// Print config as JSON
    #[structopt(short, long, group = "xor")]
    pub json: bool,

    /// Override the config file with default config
    #[structopt(long, group = "xor")]
    pub clear: bool,

    /// Override the config file with default config
    #[structopt(group = "xor")]
    pub expr: String,
}
