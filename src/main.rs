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
            cli::Tool::Claude => format_claude_code_output(&decision, command_str),
            cli::Tool::OpenCode => format_opencode_output(&decision, command_str),
        };
        println!("{}", serde_json::to_string(&output)?);
    } else {
        match &decision {
            Decision::Allow => println!("ALLOW"),
            Decision::Deny {
                message,
                match_info,
            } => {
                println!("DENY:");
                if let Some(info) = match_info {
                    println!("{}", info.format_error(command_str, message));
                } else {
                    println!("{}", message);
                }
            }
            Decision::Prompt {
                message,
                match_info,
            } => {
                println!("PROMPT:");
                if let Some(info) = match_info {
                    println!("{}", info.format_error(command_str, message));
                } else {
                    println!("{}", message);
                }
            }
        }
    }

    Ok(())
}

fn format_claude_code_output(decision: &Decision, command: &str) -> Value {
    match decision {
        Decision::Allow => serde_json::json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "allow",
                "permissionDecisionReason": "Allowed by bashguard rules"
            }
        }),
        Decision::Deny {
            message,
            match_info,
        } => {
            let reason = match match_info {
                Some(info) => info.format_error(command, message),
                None => message.clone(),
            };
            serde_json::json!({
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "deny",
                    "permissionDecisionReason": reason
                }
            })
        }
        Decision::Prompt {
            message,
            match_info,
        } => {
            let reason = match match_info {
                Some(info) => info.format_error(command, message),
                None => message.clone(),
            };
            serde_json::json!({
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "ask",
                    "permissionDecisionReason": reason
                }
            })
        }
    }
}

fn format_opencode_output(decision: &Decision, command: &str) -> Value {
    match decision {
        Decision::Allow => serde_json::json!({ "allow": true }),
        Decision::Deny {
            message,
            match_info,
        } => {
            let reason = match match_info {
                Some(info) => info.format_error(command, message),
                None => message.clone(),
            };
            serde_json::json!({ "abort": reason })
        }
        Decision::Prompt {
            message,
            match_info,
        } => {
            let reason = match match_info {
                Some(info) => info.format_error(command, message),
                None => message.clone(),
            };
            serde_json::json!({
                "abort": format!("[Requires approval] {}", reason)
            })
        }
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

    println!(
        "\nDecision: {}",
        match &decision {
            Decision::Allow => "ALLOW".to_string(),
            Decision::Deny {
                message,
                match_info,
            } => {
                let mut output = "DENY\n".to_string();
                if let Some(info) = match_info {
                    output.push_str(&info.format_error(&command, message));
                } else {
                    output.push_str(message);
                }
                output
            }
            Decision::Prompt {
                message,
                match_info,
            } => {
                let mut output = "PROMPT\n".to_string();
                if let Some(info) = match_info {
                    output.push_str(&info.format_error(&command, message));
                } else {
                    output.push_str(message);
                }
                output
            }
        }
    );

    if let Some(rule) = matched_rule {
        println!("\nMatched rule: {:?}", rule);
    } else {
        println!("\nMatched rule: (default action)");
    }

    Ok(())
}
