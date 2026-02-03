use std::{fs, path::PathBuf};

use anyhow::{Context, Result};

/// Embedded built-in profiles
const BUILTIN_PROFILES: &[(&str, &str)] = &[
    (
        "docker/read-only.toml",
        include_str!("../profiles/docker/read-only.toml"),
    ),
    (
        "general/safe-basics.toml",
        include_str!("../profiles/general/safe-basics.toml"),
    ),
    (
        "git/read-only.toml",
        include_str!("../profiles/git/read-only.toml"),
    ),
    (
        "terraform/read-only.toml",
        include_str!("../profiles/terraform/read-only.toml"),
    ),
    (
        "kubectl/read-only.toml",
        include_str!("../profiles/kubectl/read-only.toml"),
    ),
    (
        "az/read-only.toml",
        include_str!("../profiles/az/read-only.toml"),
    ),
];

/// Install built-in profiles to ~/.config/bashguard/profiles/builtins
pub fn install_builtins() -> Result<()> {
    let home = std::env::var("HOME").context("HOME environment variable not set")?;
    let builtins_dir = PathBuf::from(&home)
        .join(".config")
        .join("bashguard")
        .join("profiles")
        .join("builtins");

    println!(
        "Installing built-in profiles to: {}",
        builtins_dir.display()
    );

    for (relative_path, content) in BUILTIN_PROFILES {
        let file_path = builtins_dir.join(relative_path);

        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        fs::write(&file_path, content)
            .with_context(|| format!("Failed to write profile: {}", file_path.display()))?;
        println!("  Installed: {}", relative_path);
    }

    println!("\nBuilt-in profiles installed successfully.");
    println!("Add them to your config.toml profiles list to activate them.");
    println!("Example:");
    println!("[profiles]");
    println!("builtins = [\"git/read-only\"]");

    Ok(())
}
