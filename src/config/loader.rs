use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

use super::types::{Config, Profile, ProfileMetadata};

pub struct ConfigLoader {
    config_dir: PathBuf,
    profiles_dir: PathBuf,
}

impl ConfigLoader {
    /// Create a new config loader with default paths
    /// - Config: .claude/bashguard.toml (in current workspace)
    /// - Profiles: ~/.config/bashguard/profiles/builtins/
    pub fn new() -> Result<Self> {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        let config_dir = cwd.join(".claude");

        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        let profiles_dir = PathBuf::from(home)
            .join(".config")
            .join("bashguard")
            .join("profiles")
            .join("builtins");

        Ok(Self {
            config_dir,
            profiles_dir,
        })
    }

    /// Create a config loader with custom paths (for testing)
    pub fn with_paths(config_dir: PathBuf, profiles_dir: PathBuf) -> Self {
        Self {
            config_dir,
            profiles_dir,
        }
    }

    /// Load the main configuration and all referenced profiles
    pub fn load(&self) -> Result<Config> {
        let config_path = self.config_dir.join("bashguard.toml");

        let mut config = if config_path.exists() {
            let contents = fs::read_to_string(&config_path).with_context(|| {
                format!("Failed to read config file: {}", config_path.display())
            })?;
            toml::from_str::<Config>(&contents).with_context(|| {
                format!("Failed to parse config file: {}", config_path.display())
            })?
        } else {
            Config::default()
        };

        // Discover all available profiles
        config.available_profiles = self.discover_profiles()?;

        // Load active builtin profiles
        for profile_name in &config.profiles.builtins.clone() {
            let profile = self.load_profile_builtin(profile_name)?;
            config.loaded_profiles.push(profile);
        }

        Ok(config)
    }

    /// Discover all available profiles in the profiles directory
    fn discover_profiles(&self) -> Result<Vec<ProfileMetadata>> {
        let mut profiles = Vec::new();

        if !self.profiles_dir.exists() {
            return Ok(profiles);
        }

        self.discover_profiles_recursive(&self.profiles_dir, "", &mut profiles)?;

        Ok(profiles)
    }

    fn discover_profiles_recursive(
        &self,
        dir: &Path,
        prefix: &str,
        profiles: &mut Vec<ProfileMetadata>,
    ) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let dir_name = path.file_name().unwrap().to_string_lossy();
                let new_prefix = if prefix.is_empty() {
                    dir_name.to_string()
                } else {
                    format!("{}/{}", prefix, dir_name)
                };
                self.discover_profiles_recursive(&path, &new_prefix, profiles)?;
            } else if path.extension().is_some_and(|e| e == "toml") {
                let file_stem = path.file_stem().unwrap().to_string_lossy();
                let profile_name = if prefix.is_empty() {
                    file_stem.to_string()
                } else {
                    format!("{}/{}", prefix, file_stem)
                };

                // Try to load metadata
                let metadata = match self.load_profile_metadata(&path) {
                    Ok(m) => m,
                    Err(_) => ProfileMetadata {
                        name: profile_name.clone(),
                        description: None,
                    },
                };

                profiles.push(ProfileMetadata {
                    name: profile_name,
                    description: metadata.description,
                });
            }
        }

        Ok(())
    }

    fn load_profile_metadata(&self, path: &Path) -> Result<ProfileMetadata> {
        let contents = fs::read_to_string(path)?;
        let profile: Profile = toml::from_str(&contents)?;
        Ok(profile.profile)
    }

    /// Load a specific profile by name from builtins
    fn load_profile_builtin(&self, name: &str) -> Result<Profile> {
        // Convert profile name to path (e.g., "git/read-only" -> "git/read-only.toml")
        let profile_path = self.profiles_dir.join(format!("{}.toml", name));

        if !profile_path.exists() {
            bail!("Profile not found: {}", name);
        }

        let contents = fs::read_to_string(&profile_path)
            .with_context(|| format!("Failed to read profile: {}", name))?;
        let mut profile: Profile = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse profile: {}", name))?;
        profile.profile.name = name.to_string();

        Ok(profile)
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
        let loader =
            ConfigLoader::with_paths(temp.path().to_path_buf(), temp.path().join("profiles"));

        let config = loader.load().unwrap();
        assert!(config.profiles.builtins.is_empty());
        assert!(config.profiles.custom.is_empty());
        assert!(config.rules.is_empty());
    }

    #[test]
    fn test_discover_nested_profiles() {
        let temp = TempDir::new().unwrap();
        let profiles_dir = temp.path().join("profiles");

        // Create nested profile structure
        fs::create_dir_all(profiles_dir.join("git")).unwrap();
        fs::write(
            profiles_dir.join("git").join("read-only.toml"),
            r#"
            [profile]
            name = "git/read-only"
            description = "Read-only git operations"

            [[rules]]
            program = "git"
            subcommands = ["status"]
            action = "allow"
            "#,
        )
        .unwrap();

        let loader = ConfigLoader::with_paths(temp.path().to_path_buf(), profiles_dir);

        let config = loader.load().unwrap();
        assert_eq!(config.available_profiles.len(), 1);
        assert_eq!(config.available_profiles[0].name, "git/read-only");
    }
}
