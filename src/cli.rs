use std::{fmt, str::FromStr};

use clap::{Parser, Subcommand};

pub mod check;
pub mod init;
pub mod profiles;
pub mod test;
pub mod validate;

#[derive(Clone, Debug, Parser)]
#[command(name = "bashguard")]
#[command(about = "Rule-based permission control for Claude Code and OpenCode bash commands")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    Init(init::Args),
    Check(check::Args),
    Validate(validate::Args),
    Profiles(profiles::Args),
    Test(test::Args),
}

#[derive(Clone, Debug)]
pub enum Tool {
    Claude,
    OpenCode,
}

impl FromStr for Tool {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" => Ok(Tool::Claude),
            "opencode" => Ok(Tool::OpenCode),
            _ => Err(format!(
                "Invalid tool: '{}'. Must be 'claude' or 'opencode'.",
                s
            )),
        }
    }
}

impl fmt::Display for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tool::Claude => write!(f, "claude"),
            Tool::OpenCode => write!(f, "opencode"),
        }
    }
}
