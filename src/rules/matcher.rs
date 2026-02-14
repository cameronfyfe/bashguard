use regex::Regex;

use crate::{config::Rule, parser::ParsedCommand};

/// Information about why a rule matched
#[derive(Debug, Clone)]
pub struct MatchInfo {
    /// The reason the rule matched
    pub reason: MatchReason,
    /// Start position in the raw command string (byte offset)
    pub span_start: usize,
    /// End position in the raw command string (byte offset)
    pub span_end: usize,
}

/// The specific reason a rule matched
#[derive(Debug, Clone)]
pub enum MatchReason {
    /// Matched on program name
    Program(String),
    /// Matched on subcommands
    Subcommands(Vec<String>),
    /// Matched on a required flag being present
    FlagPresent(String),
    /// Matched on a forbidden flag being present
    FlagAbsent(String),
    /// Matched on args substring
    ArgsMatch(String),
    /// Matched on args regex
    ArgsRegex(String),
    /// Matched on working directory
    WorkingDir(String),
    /// Multiple conditions matched (reports the most specific one)
    Multiple(Box<MatchInfo>),
}

// ANSI color codes
const RED: &str = "\x1b[31m";
const BOLD_RED: &str = "\x1b[1;31m";
const RESET: &str = "\x1b[0m";

impl MatchInfo {
    /// Format the match info as a compiler-style error message
    pub fn format_error(&self, raw_command: &str, message: &str) -> String {
        let mut output = String::new();

        // Line 1: The reason description (comes first so prefixing doesn't break alignment)
        output.push_str(&self.reason_description());
        output.push('\n');

        // Line 2: The rule message (if custom)
        if !message.is_empty() && message != "Blocked by rule" {
            output.push_str("  ");
            output.push_str(message);
            output.push('\n');
        }

        // Line 3: The full command (indented) with the problematic part highlighted
        output.push_str("\n  ");
        let before = &raw_command[..self.span_start];
        let highlighted = &raw_command[self.span_start..self.span_end];
        let after = &raw_command[self.span_end..];
        output.push_str(before);
        output.push_str(BOLD_RED);
        output.push_str(highlighted);
        output.push_str(RESET);
        output.push_str(after);
        output.push('\n');

        // Line 4: Underline pointing to the problematic part (with 2-space indent to match command)
        let spaces = " ".repeat(self.span_start + 2);
        let underline_len = (self.span_end - self.span_start).max(1);
        let underline = "^".repeat(underline_len);
        output.push_str(&spaces);
        output.push_str(RED);
        output.push_str(&underline);
        output.push_str(RESET);

        output
    }

    fn reason_description(&self) -> String {
        match &self.reason {
            MatchReason::Program(name) => format!("program '{}' is not allowed", name),
            MatchReason::Subcommands(subs) => {
                format!("subcommand '{}' is not allowed", subs.join(" "))
            }
            MatchReason::FlagPresent(flag) => format!("flag '{}' is not allowed", flag),
            MatchReason::FlagAbsent(flag) => format!("missing required flag '{}'", flag),
            MatchReason::ArgsMatch(pattern) => {
                format!("argument matches blocked pattern '{}'", pattern)
            }
            MatchReason::ArgsRegex(pattern) => {
                format!("argument matches blocked regex '{}'", pattern)
            }
            MatchReason::WorkingDir(pattern) => {
                format!("working directory matches blocked pattern '{}'", pattern)
            }
            MatchReason::Multiple(inner) => inner.reason_description(),
        }
    }
}

/// Matches rules against parsed commands
pub struct RuleMatcher;

impl RuleMatcher {
    /// Check if a rule matches a parsed command
    pub fn matches(rule: &Rule, command: &ParsedCommand) -> bool {
        Self::matches_with_info(rule, command).is_some()
    }

    /// Check if a rule matches and return information about what matched
    pub fn matches_with_info(rule: &Rule, command: &ParsedCommand) -> Option<MatchInfo> {
        // Track the most specific match reason (we'll use the last one that matches)
        let mut match_info: Option<MatchInfo> = None;

        // Check program
        if let Some(ref program) = rule.program {
            if command.program != *program {
                return None;
            }
            // Find program position in raw command
            if let Some(pos) = command.raw.find(&command.program) {
                match_info = Some(MatchInfo {
                    reason: MatchReason::Program(program.clone()),
                    span_start: pos,
                    span_end: pos + command.program.len(),
                });
            }
        }

        // Check subcommands
        if !rule.subcommands.is_empty() {
            if rule.subcommands_exact {
                // Exact match: command subcommands must equal rule subcommands
                if command.subcommands != rule.subcommands {
                    return None;
                }
            } else {
                // Prefix match: command subcommands must start with rule subcommands
                if command.subcommands.len() < rule.subcommands.len() {
                    return None;
                }
                for (i, subcmd) in rule.subcommands.iter().enumerate() {
                    if command.subcommands.get(i) != Some(subcmd) {
                        return None;
                    }
                }
            }
            // Find subcommand position - look for the last subcommand in the rule
            if let Some(last_subcmd) = rule.subcommands.last() {
                if let Some(pos) = command.raw.find(last_subcmd) {
                    match_info = Some(MatchInfo {
                        reason: MatchReason::Subcommands(rule.subcommands.clone()),
                        span_start: pos,
                        span_end: pos + last_subcmd.len(),
                    });
                }
            }
        }

        // Check flags_present
        for flag in &rule.flags_present {
            if !command.flags.contains(flag) {
                return None;
            }
            // Find flag position in raw command
            if let Some(pos) = command.raw.find(flag.as_str()) {
                match_info = Some(MatchInfo {
                    reason: MatchReason::FlagPresent(flag.clone()),
                    span_start: pos,
                    span_end: pos + flag.len(),
                });
            }
        }

        // Check flags_absent - rule doesn't match if any of these flags are present
        for flag in &rule.flags_absent {
            if command.flags.contains(flag) {
                return None;
            }
        }

        // Check args_match (substring)
        if let Some(ref pattern) = rule.args_match {
            let args_str = command.args.join(" ");
            if !args_str.contains(pattern) {
                return None;
            }
            // Find the pattern in the raw command
            if let Some(pos) = command.raw.find(pattern.as_str()) {
                match_info = Some(MatchInfo {
                    reason: MatchReason::ArgsMatch(pattern.clone()),
                    span_start: pos,
                    span_end: pos + pattern.len(),
                });
            }
        }

        // Check args_regex
        if let Some(ref pattern) = rule.args_regex {
            let args_str = command.args.join(" ");
            match Regex::new(pattern) {
                Ok(re) => {
                    if let Some(m) = re.find(&args_str) {
                        // Find this match in the raw command
                        let matched_text = m.as_str();
                        if let Some(pos) = command.raw.find(matched_text) {
                            match_info = Some(MatchInfo {
                                reason: MatchReason::ArgsRegex(pattern.clone()),
                                span_start: pos,
                                span_end: pos + matched_text.len(),
                            });
                        }
                    } else {
                        return None;
                    }
                }
                Err(_) => {
                    // Invalid regex, don't match
                    return None;
                }
            }
        }

        // Check working_dir (glob pattern)
        if let Some(ref pattern) = rule.working_dir {
            if let Ok(cwd) = std::env::current_dir() {
                let cwd_str = cwd.to_string_lossy();
                match glob::Pattern::new(pattern) {
                    Ok(glob) => {
                        if !glob.matches(&cwd_str) {
                            return None;
                        }
                        // Working dir matches - point to the whole command since
                        // the issue is the context, not a specific part
                        match_info = Some(MatchInfo {
                            reason: MatchReason::WorkingDir(pattern.clone()),
                            span_start: 0,
                            span_end: command.raw.len(),
                        });
                    }
                    Err(_) => {
                        return None;
                    }
                }
            }
        }

        // If we got here, all conditions matched
        // Return match_info, or create a default one pointing to the program
        match_info.or_else(|| {
            // Default: point to the program name
            command.raw.find(&command.program).map(|pos| MatchInfo {
                reason: MatchReason::Program(command.program.clone()),
                span_start: pos,
                span_end: pos + command.program.len(),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Action;

    fn make_rule(program: Option<&str>, subcommands: Vec<&str>, action: Action) -> Rule {
        Rule {
            program: program.map(|s| s.to_string()),
            subcommands: subcommands.into_iter().map(|s| s.to_string()).collect(),
            subcommands_exact: false,
            args_match: None,
            args_regex: None,
            flags_present: vec![],
            flags_absent: vec![],
            working_dir: None,
            action,
            message: None,
        }
    }

    #[test]
    fn test_program_match() {
        let rule = make_rule(Some("git"), vec![], Action::Allow);
        let cmds = ParsedCommand::parse_all("git status").unwrap();
        assert!(RuleMatcher::matches(&rule, &cmds[0]));

        let cmds2 = ParsedCommand::parse_all("npm install").unwrap();
        assert!(!RuleMatcher::matches(&rule, &cmds2[0]));
    }

    #[test]
    fn test_subcommand_prefix_match() {
        let rule = make_rule(Some("git"), vec!["remote"], Action::Allow);

        let cmds1 = ParsedCommand::parse_all("git remote").unwrap();
        assert!(RuleMatcher::matches(&rule, &cmds1[0]));

        let cmds2 = ParsedCommand::parse_all("git remote add origin").unwrap();
        assert!(RuleMatcher::matches(&rule, &cmds2[0]));

        let cmds3 = ParsedCommand::parse_all("git status").unwrap();
        assert!(!RuleMatcher::matches(&rule, &cmds3[0]));
    }

    #[test]
    fn test_subcommand_exact_match() {
        let mut rule = make_rule(Some("git"), vec!["remote"], Action::Allow);
        rule.subcommands_exact = true;

        let cmds1 = ParsedCommand::parse_all("git remote").unwrap();
        assert!(RuleMatcher::matches(&rule, &cmds1[0]));

        let cmds2 = ParsedCommand::parse_all("git remote add origin").unwrap();
        assert!(!RuleMatcher::matches(&rule, &cmds2[0]));
    }

    #[test]
    fn test_flags_present() {
        let mut rule = make_rule(Some("git"), vec!["push"], Action::Deny);
        rule.flags_present = vec!["--force".to_string()];

        let cmds1 = ParsedCommand::parse_all("git push --force").unwrap();
        assert!(RuleMatcher::matches(&rule, &cmds1[0]));

        let cmds2 = ParsedCommand::parse_all("git push").unwrap();
        assert!(!RuleMatcher::matches(&rule, &cmds2[0]));
    }

    #[test]
    fn test_flags_absent() {
        let mut rule = make_rule(Some("git"), vec!["push"], Action::Allow);
        rule.flags_absent = vec!["--force".to_string(), "-f".to_string()];

        let cmds1 = ParsedCommand::parse_all("git push").unwrap();
        assert!(RuleMatcher::matches(&rule, &cmds1[0]));

        let cmds2 = ParsedCommand::parse_all("git push --force").unwrap();
        assert!(!RuleMatcher::matches(&rule, &cmds2[0]));

        let cmds3 = ParsedCommand::parse_all("git push -f").unwrap();
        assert!(!RuleMatcher::matches(&rule, &cmds3[0]));
    }

    #[test]
    fn test_args_regex() {
        let mut rule = make_rule(Some("rm"), vec![], Action::Deny);
        rule.args_regex = Some(r"/\*".to_string());

        let cmds1 = ParsedCommand::parse_all("rm -rf /*").unwrap();
        assert!(RuleMatcher::matches(&rule, &cmds1[0]));

        let cmds2 = ParsedCommand::parse_all("rm foo.txt").unwrap();
        assert!(!RuleMatcher::matches(&rule, &cmds2[0]));
    }
}
