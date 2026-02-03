use std::collections::{HashMap, HashSet};

use anyhow::Result;

use super::brush_adapter::parse_with_brush;

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
    /// Whether the command contains parameter expansion ($VAR, ${VAR})
    pub has_expansion: bool,
    /// Whether the command contains command substitution ($(...) or backticks)
    pub has_substitution: bool,
}

impl ParsedCommand {
    /// Parse a command string and return ALL commands found.
    ///
    /// This extracts all commands from pipelines (`cmd1 | cmd2`), chains (`cmd1 && cmd2`),
    /// and nested structures like subshells. This is the recommended method for security
    /// evaluation as it prevents bypass via: `allowed-cmd | blocked-cmd`.
    pub fn parse_all(command: &str) -> Result<Vec<Self>> {
        parse_with_brush(command)
    }

    /// Parse a command string into a single ParsedCommand.
    ///
    /// **Note:** This only returns the first command found. For security evaluation,
    /// use `parse_all()` to evaluate all commands in pipelines and chains.
    #[deprecated(
        since = "0.2.0",
        note = "Use parse_all() to evaluate all commands in pipelines and chains"
    )]
    pub fn parse(command: &str) -> Result<Self> {
        let commands = parse_with_brush(command)?;
        commands
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Empty command"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let cmds = ParsedCommand::parse_all("ls -la").unwrap();
        assert_eq!(cmds.len(), 1);
        let cmd = &cmds[0];
        assert_eq!(cmd.program, "ls");
        assert!(cmd.flags.contains("-l"));
        assert!(cmd.flags.contains("-a"));
        assert!(cmd.subcommands.is_empty());
    }

    #[test]
    fn test_git_status() {
        let cmds = ParsedCommand::parse_all("git status").unwrap();
        assert_eq!(cmds.len(), 1);
        let cmd = &cmds[0];
        assert_eq!(cmd.program, "git");
        assert_eq!(cmd.subcommands, vec!["status"]);
    }

    #[test]
    fn test_git_remote_add() {
        let cmds =
            ParsedCommand::parse_all("git remote add origin https://github.com/foo/bar").unwrap();
        assert_eq!(cmds.len(), 1);
        let cmd = &cmds[0];
        assert_eq!(cmd.program, "git");
        assert_eq!(cmd.subcommands, vec!["remote", "add"]);
        assert!(cmd.args.contains(&"origin".to_string()));
    }

    #[test]
    fn test_docker_compose_up() {
        let cmds = ParsedCommand::parse_all("docker compose up -d").unwrap();
        assert_eq!(cmds.len(), 1);
        let cmd = &cmds[0];
        assert_eq!(cmd.program, "docker");
        assert_eq!(cmd.subcommands, vec!["compose", "up"]);
        assert!(cmd.flags.contains("-d"));
    }

    #[test]
    fn test_piped_command() {
        let cmds = ParsedCommand::parse_all("ls | grep foo").unwrap();
        // With brush-parser, we now get ALL commands in the pipeline
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].program, "ls");
        assert_eq!(cmds[1].program, "grep");
        assert!(cmds[0].is_piped);
        assert!(cmds[1].is_piped);
    }

    #[test]
    fn test_env_vars() {
        let cmds = ParsedCommand::parse_all("NODE_ENV=production npm start").unwrap();
        assert_eq!(cmds.len(), 1);
        let cmd = &cmds[0];
        assert_eq!(cmd.program, "npm");
        assert_eq!(
            cmd.env_vars.get("NODE_ENV"),
            Some(&"production".to_string())
        );
    }

    #[test]
    fn test_pipeline_all_commands() {
        let cmds = ParsedCommand::parse_all("ls | grep foo | wc -l").unwrap();
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0].program, "ls");
        assert_eq!(cmds[1].program, "grep");
        assert_eq!(cmds[2].program, "wc");
    }

    #[test]
    fn test_chain_all_commands() {
        let cmds = ParsedCommand::parse_all("cd /tmp && ls -la || echo failed").unwrap();
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0].program, "cd");
        assert_eq!(cmds[1].program, "ls");
        assert_eq!(cmds[2].program, "echo");
    }
}
