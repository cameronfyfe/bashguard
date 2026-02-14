use clap::Parser;

/// List all known capabilities from command maps
#[derive(Clone, Debug, Parser)]
pub struct Args {
    /// Output format: text or toml
    #[clap(short, long, default_value = "text")]
    pub format: String,
}
