use clap::Parser;

use crate::cli::Tool;

/// Check a command against current rules (reads from stdin, used by hooks)
#[derive(Clone, Debug, Parser)]
pub struct Args {
    /// Output in JSON format
    #[clap(long)]
    pub json: bool,

    /// Output format (required)
    #[clap(long)]
    pub format: Tool,
}
