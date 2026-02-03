use std::collections::{HashMap, HashSet};

use anyhow::{bail, Result};

use super::{
    lexer::{Lexer, Token},
    semantic::SemanticAnalyzer,
};

/// A parsed shell command with semantic information
#[derive(Debug, Clone)]
pub struct ParsedCommand {
    /// The raw command string
    pub raw: String,
    /// The program being invoked (e.g., "git")
    pub program: String,
    /// Chained subcommands (e.g., ["remote", "add"])
    pub subcommands: Vec<String>,
    /// Positional arguments
    pub args: Vec<String>,
    /// Flags (both short and long, e.g., "-f", "--force")
    pub flags: HashSet<String>,
    /// Whether the command contains a pipe
    pub is_piped: bool,
    /// Whether the command has output redirection
    pub has_redirect: bool,
    /// Environment variables set before the command
    pub env_vars: HashMap<String, String>,
}

impl ParsedCommand {
    /// Parse a command string into a ParsedCommand
    pub fn parse(command: &str) -> Result<Self> {
        let mut lexer = Lexer::new(command);
        let tokens = lexer.tokenize()?;

        if tokens.is_empty() {
            bail!("Empty command");
        }

        // Extract env vars
        let mut env_vars = HashMap::new();
        let mut cmd_start = 0;

        for (i, token) in tokens.iter().enumerate() {
            if let Token::EnvVar(key, value) = token {
                env_vars.insert(key.clone(), value.clone());
                cmd_start = i + 1;
            } else {
                break;
            }
        }

        // Check for pipes and redirects
        let is_piped = tokens.iter().any(|t| matches!(t, Token::Pipe));
        let has_redirect = tokens.iter().any(|t| {
            matches!(
                t,
                Token::RedirectOut | Token::RedirectAppend | Token::RedirectIn
            )
        });

        // Get the words for the first command (before any pipe/redirect/operator)
        let words: Vec<String> = tokens[cmd_start..]
            .iter()
            .take_while(|t| {
                !matches!(
                    t,
                    Token::Pipe
                        | Token::RedirectOut
                        | Token::RedirectAppend
                        | Token::RedirectIn
                        | Token::And
                        | Token::Or
                        | Token::Semicolon
                        | Token::Background
                )
            })
            .filter_map(|t| {
                if let Token::Word(w) = t {
                    Some(w.clone())
                } else {
                    None
                }
            })
            .collect();

        if words.is_empty() {
            bail!("Empty command");
        }

        let program = words[0].clone();
        let remaining: Vec<String> = words[1..].to_vec();

        // Use semantic analyzer to extract subcommands, flags, and args
        let analyzer = SemanticAnalyzer::new();
        let (subcommands, flags, args) = analyzer.analyze(&program, &remaining);

        Ok(ParsedCommand {
            raw: command.to_string(),
            program,
            subcommands,
            args,
            flags,
            is_piped,
            has_redirect,
            env_vars,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let cmd = ParsedCommand::parse("ls -la").unwrap();
        assert_eq!(cmd.program, "ls");
        assert!(cmd.flags.contains("-l"));
        assert!(cmd.flags.contains("-a"));
        assert!(cmd.subcommands.is_empty());
    }

    #[test]
    fn test_git_status() {
        let cmd = ParsedCommand::parse("git status").unwrap();
        assert_eq!(cmd.program, "git");
        assert_eq!(cmd.subcommands, vec!["status"]);
    }

    #[test]
    fn test_git_remote_add() {
        let cmd = ParsedCommand::parse("git remote add origin https://github.com/foo/bar").unwrap();
        assert_eq!(cmd.program, "git");
        assert_eq!(cmd.subcommands, vec!["remote", "add"]);
        assert!(cmd.args.contains(&"origin".to_string()));
    }

    #[test]
    fn test_docker_compose_up() {
        let cmd = ParsedCommand::parse("docker compose up -d").unwrap();
        assert_eq!(cmd.program, "docker");
        assert_eq!(cmd.subcommands, vec!["compose", "up"]);
        assert!(cmd.flags.contains("-d"));
    }

    #[test]
    fn test_piped_command() {
        let cmd = ParsedCommand::parse("ls | grep foo").unwrap();
        assert_eq!(cmd.program, "ls");
        assert!(cmd.is_piped);
    }

    #[test]
    fn test_env_vars() {
        let cmd = ParsedCommand::parse("NODE_ENV=production npm start").unwrap();
        assert_eq!(cmd.program, "npm");
        assert_eq!(
            cmd.env_vars.get("NODE_ENV"),
            Some(&"production".to_string())
        );
    }
}
