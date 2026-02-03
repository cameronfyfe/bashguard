use clap::Parser;

use crate::cli::Tool;

/// Initialize bashguard in the current repository
#[derive(Clone, Debug, Parser)]
pub struct Args {
    /// Target tool to integrate with (required)
    #[clap(short, long)]
    pub tool: Tool,
}
