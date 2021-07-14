//! `keys` subcommand
use abscissa_core::{Command, Help, Options, Runnable};

mod add;
mod clean;
mod list;
mod restore;

/// `keys` subcommand
#[derive(Command, Debug, Options, Runnable)]
pub enum KeysCmd {
    /// The `help` subcommand
    #[options(help = "Get usage information")]
    Help(Help<Self>),

    /// The `keys add` subcommand
    #[options(help = "Adds a key to a configured chain")]
    Add(add::KeysAddCmd),

    /// The `keys clean` subcommand
    #[options(help = "Removes a key from a configured chain")]
    Clean(clean::KeysCleanCmd),

    /// The `keys list` subcommand
    #[options(help = "List keys configured on a chain")]
    List(list::KeysListCmd),

    /// The `keys restore` subcommand
    #[options(help = "restore a key to a configured chain using a mnemonic")]
    Restore(restore::KeyRestoreCmd),
}
