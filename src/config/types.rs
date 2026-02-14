use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// The main configuration structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub settings: Settings,

    /// Global capability policies (capability -> action)
    /// Example: "git.force_push" = "deny"
    #[serde(default)]
    pub capabilities: HashMap<String, Action>,

    #[serde(default)]
    pub rules: Vec<Rule>,
}

/// Global settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Default action when no rule matches
    #[serde(default = "default_action")]
    pub default_action: Action,

    /// Whether to log decisions
    #[serde(default)]
    pub log_decisions: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_action: Action::Prompt,
            log_decisions: false,
        }
    }
}

fn default_action() -> Action {
    Action::Prompt
}

/// A rule that matches commands and specifies an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Program name to match (e.g., "git")
    #[serde(default)]
    pub program: Option<String>,

    /// Subcommands to match (e.g., ["remote", "add"])
    #[serde(default)]
    pub subcommands: Vec<String>,

    /// If true, subcommands must match exactly; if false, prefix match
    #[serde(default)]
    pub subcommands_exact: bool,

    /// Substring to match in args
    #[serde(default)]
    pub args_match: Option<String>,

    /// Regex to match in args
    #[serde(default)]
    pub args_regex: Option<String>,

    /// Flags that must be present
    #[serde(default)]
    pub flags_present: Vec<String>,

    /// Flags that must be absent
    #[serde(default)]
    pub flags_absent: Vec<String>,

    /// Glob pattern for working directory
    #[serde(default)]
    pub working_dir: Option<String>,

    /// Action to take when rule matches
    pub action: Action,

    /// Message to display on deny/prompt
    #[serde(default)]
    pub message: Option<String>,
}

/// Action to take for a command
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Allow,
    Deny,
    Prompt,
}
