use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use crate::cli;

const DEFAULT_CONFIG: &str = include_str!("./templates/config.toml");
const OPENCODE_PLUGIN_TEMPLATE: &str = include_str!("./templates/opencodePlugin.ts");

/// Initialize bashguard in the current repository
pub fn init(args: cli::init::Args) -> Result<()> {
    let cli::init::Args { tool } = args;

    let cwd = env::current_dir().context("Failed to get current directory")?;

    // Check if we're in a git repo
    let git_dir = cwd.join(".git");
    if !git_dir.exists() {
        bail!(
            "Not in a git repository.\n\
             Run this command from the root of your project."
        );
    }

    // Create .bashguard directory and config
    let bashguard_dir = cwd.join(".bashguard");
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

    // Tool-specific initialization
    match tool {
        cli::Tool::Claude => init_claude_code(&cwd)?,
        cli::Tool::OpenCode => init_opencode(&cwd)?,
    }

    println!("\nBashguard initialized successfully for {tool}!");
    println!("\nNext steps:");
    println!("  1. Install built-in profiles: bashguard profiles install-builtins");
    println!("  2. Edit .bashguard/config.toml to configure rules");

    Ok(())
}

/// Initialize Claude Code integration
fn init_claude_code(cwd: &Path) -> Result<()> {
    let claude_dir = cwd.join(".claude");
    fs::create_dir_all(&claude_dir)
        .with_context(|| format!("Failed to create directory: {}", claude_dir.display()))?;

    let settings_path = claude_dir.join("settings.local.json");
    update_claude_settings(&settings_path)?;

    Ok(())
}

fn update_claude_settings(settings_path: &PathBuf) -> Result<()> {
    let bashguard_hook_entry = json!({
        "matcher": "Bash",
        "hooks": [{ "type": "command", "command": "bashguard check --json --format claude" }]
    });

    let mut settings: Value = if settings_path.exists() {
        let contents = fs::read_to_string(settings_path)
            .with_context(|| format!("Failed to read: {}", settings_path.display()))?;
        serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse: {}", settings_path.display()))?
    } else {
        json!({})
    };

    // Get or create hooks object with PreToolUse array
    let hooks = settings.get_mut("hooks").and_then(|h| h.as_object_mut());

    // Check if bashguard hook already exists in PreToolUse
    let has_bashguard = hooks
        .as_ref()
        .and_then(|h| h.get("PreToolUse"))
        .and_then(|arr| arr.as_array())
        .map(|arr| {
            arr.iter().any(|entry| {
                entry
                    .get("hooks")
                    .and_then(|h| h.as_array())
                    .map(|hooks| {
                        hooks.iter().any(|hook| {
                            hook.get("command")
                                .and_then(|c| c.as_str())
                                .map(|c| c.contains("bashguard"))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    if has_bashguard {
        println!("Hook already configured: {}", settings_path.display());
        return Ok(());
    }

    // Initialize hooks structure if needed
    if !settings
        .get("hooks")
        .map(|h| h.is_object())
        .unwrap_or(false)
    {
        settings["hooks"] = json!({});
    }

    // Get or create PreToolUse array
    let hooks_obj = settings["hooks"].as_object_mut().unwrap();
    let is_new = !hooks_obj.contains_key("PreToolUse");

    if is_new {
        hooks_obj.insert("PreToolUse".to_string(), json!([]));
    }

    // Add bashguard hook entry to PreToolUse array
    let pre_tool_use = hooks_obj
        .get_mut("PreToolUse")
        .unwrap()
        .as_array_mut()
        .unwrap();
    pre_tool_use.push(bashguard_hook_entry);

    if is_new {
        println!("Created hook in: {}", settings_path.display());
    } else {
        println!("Updated hook in: {}", settings_path.display());
    }

    // Write updated settings
    let output = serde_json::to_string_pretty(&settings)?;
    fs::write(settings_path, output)
        .with_context(|| format!("Failed to write: {}", settings_path.display()))?;

    Ok(())
}

/// Initialize OpenCode integration
fn init_opencode(cwd: &Path) -> Result<()> {
    let opencode_dir = cwd.join(".opencode");
    let plugins_dir = opencode_dir.join("plugins");
    fs::create_dir_all(&plugins_dir)
        .with_context(|| format!("Failed to create directory: {}", plugins_dir.display()))?;

    let plugin_path = plugins_dir.join("bashguard.ts");
    if plugin_path.exists() {
        println!("Plugin already exists: {}", plugin_path.display());
    } else {
        fs::write(&plugin_path, OPENCODE_PLUGIN_TEMPLATE)
            .with_context(|| format!("Failed to write plugin: {}", plugin_path.display()))?;
        println!("Created plugin: {}", plugin_path.display());
    }

    Ok(())
}
