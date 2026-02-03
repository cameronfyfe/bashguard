use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bashguard")]
#[command(about = "Rule-based permission control for Claude Code bash commands")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize bashguard in the current repository
    Init,
    /// Check a command against current rules (reads from stdin, used by Claude Code hook)
    Check {
        /// Output in JSON format
        #[clap(long)]
        json: bool,
    },
    /// Validate current configuration files
    Validate,
    /// Manage profiles
    Profiles {
        #[command(subcommand)]
        command: ProfilesCommand,
    },
    /// Test a command against current rules
    Test {
        /// The command to test
        #[clap(short, long)]
        command: String,
    },
}

#[derive(Subcommand)]
pub enum ProfilesCommand {
    /// Copy built-in profiles to ~/.config/bashguard/profiles/builtins
    InstallBuiltins,
}
