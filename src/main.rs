use std::{
    io::{self, BufRead},
    process::exit,
};

use anyhow::{Context, Result};
use bashguard::{
    cli::{self, Cli, Command},
    Config, Decision, Evaluator, ParsedCommand, SessionLogger,
};
use clap::Parser;
use serde_json::Value;

mod init;
mod profiles;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Init(args) => init::init(args),
        Command::Check(args) => check(args),
        Command::Validate(args) => validate(args),
        Command::Profiles(args) => profiles(args),
        Command::Test(args) => test(args),
    };

    if let Err(e) = result {
        eprintln!("[bashguard] Error: {}", e);
        exit(1);
    }
}

fn check(args: cli::check::Args) -> Result<()> {
    let cli::check::Args { json, format } = args;

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
    // Parse ALL commands in the input (handles pipelines, chains, etc.)
    let parsed_commands = ParsedCommand::parse_all(command_str)?;
    let evaluator = Evaluator::new(&config);
    // Evaluate ALL commands - strictest decision wins
    let (decision, matched_rule) = evaluator.evaluate_all_with_trace(&parsed_commands);

    // Log using the first parsed command for display (the raw command is still logged)
    let logger = SessionLogger::new();
    if let Some(first_parsed) = parsed_commands.first() {
        if let Err(e) = logger.log_action(
            session_id,
            command_str,
            first_parsed,
            &decision,
            matched_rule.as_ref(),
        ) {
            eprintln!("[bashguard] Failed to log action: {}", e);
        }
    }

    if json {
        let output = match format {
            cli::Tool::Claude => format_claude_code_output(&decision),
            cli::Tool::OpenCode => format_opencode_output(&decision),
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

fn format_claude_code_output(decision: &Decision) -> Value {
    match decision {
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
    }
}

fn format_opencode_output(decision: &Decision) -> Value {
    match decision {
        Decision::Allow => serde_json::json!({ "allow": true }),
        Decision::Deny { message } => serde_json::json!({ "abort": message }),
        Decision::Prompt { message } => serde_json::json!({
            "abort": format!("[Requires approval] {}", message)
        }),
    }
}

fn validate(args: cli::validate::Args) -> Result<()> {
    let _ = args;

    Config::load()?;

    println!("Configuration is valid.");

    Ok(())
}

fn profiles(args: cli::profiles::Args) -> Result<()> {
    match args.command {
        cli::profiles::Command::InstallBuiltins(args) => profiles::install_builtins(args),
    }
}

fn test(args: cli::test::Args) -> Result<()> {
    let cli::test::Args { command } = args;

    let config = Config::load()?;
    // Parse ALL commands in the input
    let parsed_commands = ParsedCommand::parse_all(&command)?;
    let evaluator = Evaluator::new(&config);
    // Evaluate ALL commands
    let (decision, matched_rule) = evaluator.evaluate_all_with_trace(&parsed_commands);

    println!("Command: {}", command);
    println!("\nParsed ({} command(s)):", parsed_commands.len());
    for (i, parsed) in parsed_commands.iter().enumerate() {
        println!("  [{}] Program: {}", i + 1, parsed.program);
        println!("      Subcommands: {:?}", parsed.subcommands);
        println!("      Flags: {:?}", parsed.flags);
        println!("      Args: {:?}", parsed.args);
        if parsed.has_expansion {
            println!("      Has expansion: yes");
        }
        if parsed.has_substitution {
            println!("      Has substitution: yes");
        }
    }

    println!("\nOverall Decision: {:?}", decision);
    if let Some(rule) = matched_rule {
        println!("Matched rule: {:?}", rule);
    } else {
        println!("Matched rule: (default action)");
    }

    Ok(())
}
