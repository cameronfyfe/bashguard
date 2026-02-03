use std::{
    io::{self, BufRead},
    process::exit,
};

use anyhow::{Context, Result};
use bashguard::{
    cli::{Cli, Command, ProfilesCommand},
    Config, Decision, Evaluator, ParsedCommand, SessionLogger,
};
use clap::Parser;
use serde_json::Value;

mod init;
mod profiles;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Init => init::init(),
        Command::Check { json } => check(json),
        Command::Validate => validate(),
        Command::Profiles { command } => match command {
            ProfilesCommand::InstallBuiltins => profiles::install_builtins(),
        },
        Command::Test { command } => test(&command),
    };

    if let Err(e) = result {
        eprintln!("[bashguard] Error: {}", e);
        exit(1);
    }
}

fn check(json_output: bool) -> Result<()> {
    let stdin = io::stdin();
    let input: String = stdin
        .lock()
        .lines()
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");

    let hook_input = serde_json::from_str::<Value>(&input)?;
    let command_str = hook_input["tool_input"]["command"]
        .as_str()
        .context("Missing command in tool_input")?;

    let session_id = hook_input["session_id"]
        .as_str()
        .unwrap_or("unknown-session");

    let config = Config::load()?;
    let parsed = ParsedCommand::parse(command_str)?;
    let evaluator = Evaluator::new(&config);
    let (decision, matched_rule) = evaluator.evaluate_with_trace(&parsed);

    let logger = SessionLogger::new();
    if let Err(e) = logger.log_action(
        session_id,
        command_str,
        &parsed,
        &decision,
        matched_rule.as_ref(),
    ) {
        eprintln!("[bashguard] Failed to log action: {}", e);
    }

    if json_output {
        let output = match &decision {
            Decision::Allow => serde_json::json!({
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "allow",
                    "permissionDecisionReason": "Allowed by bashguard rules"
                }
            }),
            Decision::Deny { message } => serde_json::json!({
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "deny",
                    "permissionDecisionReason": message
                }
            }),
            Decision::Prompt { message } => serde_json::json!({
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "ask",
                    "permissionDecisionReason": message
                }
            }),
        };
        println!("{}", serde_json::to_string(&output)?);
    } else {
        match &decision {
            Decision::Allow => println!("ALLOW"),
            Decision::Deny { message } => println!("DENY: {}", message),
            Decision::Prompt { message } => println!("PROMPT: {}", message),
        }
    }

    Ok(())
}

fn validate() -> Result<()> {
    Config::load()?;

    println!("Configuration is valid.");

    Ok(())
}

fn test(command: &str) -> Result<()> {
    let config = Config::load()?;
    let parsed = ParsedCommand::parse(command)?;
    let evaluator = Evaluator::new(&config);
    let (decision, matched_rule) = evaluator.evaluate_with_trace(&parsed);

    println!("Command: {}", command);
    println!("\nParsed:");
    println!("  Program: {}", parsed.program);
    println!("  Subcommands: {:?}", parsed.subcommands);
    println!("  Flags: {:?}", parsed.flags);
    println!("  Args: {:?}", parsed.args);

    println!("\nDecision: {:?}", decision);
    if let Some(rule) = matched_rule {
        println!("Matched rule: {:?}", rule);
    } else {
        println!("Matched rule: (default action)");
    }

    Ok(())
}
