use serde::{Deserialize, Serialize};

/// The main configuration structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub settings: Settings,

    #[serde(default)]
    pub profiles: ProfilesConfig,

    #[serde(default)]
    pub rules: Vec<Rule>,

    /// Loaded profile data (populated by loader)
    #[serde(skip)]
    pub loaded_profiles: Vec<Profile>,

    /// All available profiles (populated by loader)
    #[serde(skip)]
    pub available_profiles: Vec<ProfileMetadata>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfilesConfig {
    #[serde(default)]
    pub builtins: Vec<String>,

    #[serde(default)]
    pub custom: Vec<String>,
}

impl Config {
    /// Get list of available profiles
    pub fn available_profiles(&self) -> &[ProfileMetadata] {
        &self.available_profiles
    }

    /// Check if a profile is currently active
    pub fn is_profile_active(&self, name: &str) -> bool {
        self.profiles.builtins.iter().any(|p| p == name)
            || self.profiles.custom.iter().any(|p| p == name)
    }
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

/// A profile containing a set of rules
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Profile {
    #[serde(default)]
    pub profile: ProfileMetadata,

    #[serde(default)]
    pub rules: Vec<Rule>,
}

/// Profile metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfileMetadata {
    /// Profile name (e.g., "git/read-only")
    #[serde(default)]
    pub name: String,

    /// Profile description
    #[serde(default)]
    pub description: Option<String>,
}
