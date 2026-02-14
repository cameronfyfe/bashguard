use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use super::types::Config;

pub struct ConfigLoader {
    config_dir: PathBuf,
}

impl ConfigLoader {
    /// Create a new config loader with default paths
    /// - Config: .bashguard/config.toml (in current workspace)
    pub fn new() -> Result<Self> {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        let config_dir = Self::find_config_dir(&cwd);

        Ok(Self { config_dir })
    }

    fn find_config_dir(start: &Path) -> PathBuf {
        for ancestor in start.ancestors() {
            let candidate = ancestor.join(".bashguard").join("config.toml");
            if candidate.exists() {
                return ancestor.join(".bashguard");
            }
        }

        start.join(".bashguard")
    }

    /// Create a config loader with custom path (for testing)
    pub fn with_path(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    /// Load the main configuration
    pub fn load(&self) -> Result<Config> {
        let config_path = self.config_dir.join("config.toml");

        let config = if config_path.exists() {
            let contents = fs::read_to_string(&config_path).with_context(|| {
                format!("Failed to read config file: {}", config_path.display())
            })?;
            toml::from_str::<Config>(&contents).with_context(|| {
                format!("Failed to parse config file: {}", config_path.display())
            })?
        } else {
            Config::default()
        };

        Ok(config)
    }
}

impl Config {
    /// Load configuration from default location
    pub fn load() -> Result<Self> {
        ConfigLoader::new()?.load()
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_empty_config() {
        let temp = TempDir::new().unwrap();
        let loader = ConfigLoader::with_path(temp.path().to_path_buf());

        let config = loader.load().unwrap();
        assert!(config.capabilities.is_empty());
        assert!(config.rules.is_empty());
    }

    #[test]
    fn test_load_capabilities() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("config.toml"),
            r#"
            [capabilities]
            "git.force_push" = "deny"
            "#,
        )
        .unwrap();

        let loader = ConfigLoader::with_path(temp.path().to_path_buf());

        let config = loader.load().unwrap();
        assert_eq!(
            config.capabilities.get("git.force_push"),
            Some(&super::super::Action::Deny)
        );
    }

    #[test]
    fn test_find_config_dir_in_parent() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let config_dir = root.join(".bashguard");
        let nested = root.join("nested").join("dir");

        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join("config.toml"), "").unwrap();
        fs::create_dir_all(&nested).unwrap();

        let found = ConfigLoader::find_config_dir(&nested);
        assert_eq!(found, config_dir);
    }
}
