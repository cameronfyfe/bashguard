use clap::Parser;

pub mod install_builtins;

/// Manage profiles
#[derive(Clone, Debug, Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Clone, Debug, Parser)]
pub enum Command {
    /// Copy built-in profiles to ~/.config/bashguard/profiles/builtins
    InstallBuiltins(install_builtins::Args),
}
