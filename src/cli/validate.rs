use clap::Parser;

/// Validate current configuration files
#[derive(Clone, Debug, Parser)]
pub struct Args {
    /// The command to test
    #[clap(short, long)]
    pub command: String,
}
