use std::{env, fs, path::PathBuf};

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

const DEFAULT_CONFIG: &str = r#"# Bashguard configuration
# See: https://github.com/cameronfyfe/bashguard

[profiles]
# Built-in profiles to activate (run `bashguard profiles install-builtins` first)
# Examples: "git/read-only", "docker/read-only", "kubectl/read-only"
builtins = ["general/safe-basics"]

# Custom profile files (relative to ~/.config/bashguard/profiles/)
custom = []

[settings]
# Default action for commands that don't match any rule
# Options: "allow", "deny", "prompt"
default_action = "prompt"

# Log all decisions to .claude/bashguard/logs/
log_decisions = true

# Inline rules (highest priority, evaluated before profiles)
# [[rules]]
# program = "rm"
# flags_present = ["-rf"]
# action = "deny"
# message = "Recursive force delete is not allowed"
"#;

/// Initialize bashguard in the current repository
pub fn init() -> Result<()> {
    let cwd = env::current_dir().context("Failed to get current directory")?;

    // Check if we're in a git repo or have a .claude directory
    let claude_dir = cwd.join(".claude");
    let git_dir = cwd.join(".git");

    if !git_dir.exists() && !claude_dir.exists() {
        bail!(
            "Not in a git repository and no .claude directory found.\n\
             Run this command from the root of your project."
        );
    }

    // Create .claude/bashguard directory
    let bashguard_dir = claude_dir.join("bashguard");
    fs::create_dir_all(&bashguard_dir)
        .with_context(|| format!("Failed to create directory: {}", bashguard_dir.display()))?;

    // Create config.toml
    let config_path = bashguard_dir.join("config.toml");
    if config_path.exists() {
        println!("Config already exists: {}", config_path.display());
    } else {
        fs::write(&config_path, DEFAULT_CONFIG)
            .with_context(|| format!("Failed to write config: {}", config_path.display()))?;
        println!("Created config: {}", config_path.display());
    }

    // Create or update .claude/settings.local.json with the hook
    let settings_path = claude_dir.join("settings.local.json");
    update_claude_settings(&settings_path)?;

    println!("\nBashguard initialized successfully!");
    println!("\nNext steps:");
    println!("  1. Install built-in profiles: bashguard profiles install-builtins");
    println!("  2. Edit .claude/bashguard/config.toml to configure rules");

    Ok(())
}

fn update_claude_settings(settings_path: &PathBuf) -> Result<()> {
    let bashguard_hook = json!({
        "type": "command",
        "command": "bashguard check --json"
    });

    let mut settings: Value = if settings_path.exists() {
        let contents = fs::read_to_string(settings_path)
            .with_context(|| format!("Failed to read: {}", settings_path.display()))?;
        serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse: {}", settings_path.display()))?
    } else {
        json!({})
    };

    // Ensure hooks object exists
    if settings.get("hooks").is_none() {
        settings["hooks"] = json!({});
    }

    // Check if PreToolUse hook already exists
    let hooks = settings["hooks"].as_object_mut().unwrap();

    if let Some(existing) = hooks.get("PreToolUse") {
        // Check if it's already configured for bashguard
        if let Some(arr) = existing.as_array() {
            let has_bashguard = arr.iter().any(|h| {
                h.get("command")
                    .and_then(|c| c.as_str())
                    .map(|c| c.contains("bashguard"))
                    .unwrap_or(false)
            });

            if has_bashguard {
                println!("Hook already configured: {}", settings_path.display());
                return Ok(());
            }

            // Add bashguard to existing hooks
            let mut new_hooks = arr.clone();
            new_hooks.push(bashguard_hook);
            hooks.insert("PreToolUse".to_string(), Value::Array(new_hooks));
        } else {
            // Single hook exists, convert to array
            let existing_hook = existing.clone();
            hooks.insert(
                "PreToolUse".to_string(),
                json!([existing_hook, bashguard_hook]),
            );
        }
        println!("Updated hook in: {}", settings_path.display());
    } else {
        // No PreToolUse hook, create new one
        hooks.insert("PreToolUse".to_string(), json!([bashguard_hook]));
        println!("Created hook in: {}", settings_path.display());
    }

    // Write updated settings
    let output = serde_json::to_string_pretty(&settings)?;
    fs::write(settings_path, output)
        .with_context(|| format!("Failed to write: {}", settings_path.display()))?;

    Ok(())
}
