use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{Decision, ParsedCommand};

/// Log entry for a hook action
#[derive(Debug, Serialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub command: String,
    pub parsed: ParsedCommandLog,
    pub decision: String,
    pub decision_reason: Option<String>,
    pub matched_rule: Option<String>,
}

/// Simplified parsed command for logging
#[derive(Debug, Serialize)]
pub struct ParsedCommandLog {
    pub program: String,
    pub subcommands: Vec<String>,
    pub flags: Vec<String>,
    pub args: Vec<String>,
}

impl From<&ParsedCommand> for ParsedCommandLog {
    fn from(cmd: &ParsedCommand) -> Self {
        Self {
            program: cmd.program.clone(),
            subcommands: cmd.subcommands.clone(),
            flags: cmd.flags.iter().cloned().collect(),
            args: cmd.args.clone(),
        }
    }
}

/// Logger that writes to session-specific log files
pub struct SessionLogger {
    log_dir: PathBuf,
}

impl SessionLogger {
    /// Create a new session logger (logs to current workspace)
    pub fn new() -> Self {
        let log_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".claude")
            .join("bashguard")
            .join("logs");

        Self { log_dir }
    }

    /// Ensure the log directory exists
    fn ensure_log_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.log_dir)
    }

    /// Get the log file path for a session
    fn log_file_path(&self, session_id: &str) -> PathBuf {
        // Sanitize session ID to be safe for filenames
        let safe_session_id: String = session_id
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        self.log_dir.join(format!("{}.jsonl", safe_session_id))
    }

    /// Log a hook action
    pub fn log_action(
        &self,
        session_id: &str,
        command: &str,
        parsed: &ParsedCommand,
        decision: &Decision,
        matched_rule: Option<&crate::config::Rule>,
    ) -> std::io::Result<()> {
        self.ensure_log_dir()?;

        let (decision_str, reason) = match decision {
            Decision::Allow => ("allow".to_string(), None),
            Decision::Deny { message } => ("deny".to_string(), Some(message.clone())),
            Decision::Prompt { message } => ("prompt".to_string(), Some(message.clone())),
        };

        let entry = LogEntry {
            timestamp: Utc::now(),
            session_id: session_id.to_string(),
            command: command.to_string(),
            parsed: ParsedCommandLog::from(parsed),
            decision: decision_str,
            decision_reason: reason,
            matched_rule: matched_rule.map(|r| format!("{:?}", r)),
        };

        let log_path = self.log_file_path(session_id);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        let json = serde_json::to_string(&entry).map_err(std::io::Error::other)?;

        writeln!(file, "{}", json)?;

        Ok(())
    }
}

impl Default for SessionLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_log_file_path_sanitization() {
        let logger = SessionLogger::new();

        // Normal session ID
        let path = logger.log_file_path("abc-123");
        assert!(path.to_string_lossy().ends_with("abc-123.jsonl"));

        // Session ID with special characters
        let path = logger.log_file_path("session/with:special<chars>");
        assert!(path
            .to_string_lossy()
            .ends_with("session_with_special_chars_.jsonl"));
    }

    #[test]
    fn test_log_action() {
        let temp_dir = TempDir::new().unwrap();
        let mut logger = SessionLogger::new();
        logger.log_dir = temp_dir.path().to_path_buf();

        let parsed = ParsedCommand::parse("git status").unwrap();
        let decision = Decision::Allow;

        logger
            .log_action("test-session", "git status", &parsed, &decision, None)
            .unwrap();

        let log_path = logger.log_file_path("test-session");
        assert!(log_path.exists());

        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("git status"));
        assert!(content.contains("test-session"));
        assert!(content.contains("\"decision\":\"allow\""));
    }
}
