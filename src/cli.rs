use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bashguard")]
#[command(about = "Rule-based permission control for Claude Code and OpenCode bash commands")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize bashguard in the current repository
    Init {
        /// Target tool to integrate with (required): "claude" or "opencode"
        #[clap(long)]
        tool: String,
    },
    /// Check a command against current rules (reads from stdin, used by hooks)
    Check {
        /// Output in JSON format
        #[clap(long)]
        json: bool,

        /// Output format (required): "claude" or "opencode"
        #[clap(long)]
        format: String,
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
