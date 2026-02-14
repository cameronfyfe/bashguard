use clap::Parser;

/// Test a command against current policy
#[derive(Clone, Debug, Parser)]
pub struct Args {
    /// The command to test
    #[clap(short, long)]
    pub command: String,
}
