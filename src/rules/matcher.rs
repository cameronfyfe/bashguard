use regex::Regex;

use crate::{config::Rule, parser::ParsedCommand};

/// Matches rules against parsed commands
pub struct RuleMatcher;

impl RuleMatcher {
    /// Check if a rule matches a parsed command
    pub fn matches(rule: &Rule, command: &ParsedCommand) -> bool {
        // Check program
        if let Some(ref program) = rule.program {
            if command.program != *program {
                return false;
            }
        }

        // Check subcommands
        if !rule.subcommands.is_empty() {
            if rule.subcommands_exact {
                // Exact match: command subcommands must equal rule subcommands
                if command.subcommands != rule.subcommands {
                    return false;
                }
            } else {
                // Prefix match: command subcommands must start with rule subcommands
                if command.subcommands.len() < rule.subcommands.len() {
                    return false;
                }
                for (i, subcmd) in rule.subcommands.iter().enumerate() {
                    if command.subcommands.get(i) != Some(subcmd) {
                        return false;
                    }
                }
            }
        }

        // Check flags_present
        for flag in &rule.flags_present {
            if !command.flags.contains(flag) {
                return false;
            }
        }

        // Check flags_absent
        for flag in &rule.flags_absent {
            if command.flags.contains(flag) {
                return false;
            }
        }

        // Check args_match (substring)
        if let Some(ref pattern) = rule.args_match {
            let args_str = command.args.join(" ");
            if !args_str.contains(pattern) {
                return false;
            }
        }

        // Check args_regex
        if let Some(ref pattern) = rule.args_regex {
            let args_str = command.args.join(" ");
            match Regex::new(pattern) {
                Ok(re) => {
                    if !re.is_match(&args_str) {
                        return false;
                    }
                }
                Err(_) => {
                    // Invalid regex, don't match
                    return false;
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
                            return false;
                        }
                    }
                    Err(_) => {
                        return false;
                    }
                }
            }
        }

        true
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
        let cmd = ParsedCommand::parse("git status").unwrap();
        assert!(RuleMatcher::matches(&rule, &cmd));

        let cmd2 = ParsedCommand::parse("npm install").unwrap();
        assert!(!RuleMatcher::matches(&rule, &cmd2));
    }

    #[test]
    fn test_subcommand_prefix_match() {
        let rule = make_rule(Some("git"), vec!["remote"], Action::Allow);

        let cmd1 = ParsedCommand::parse("git remote").unwrap();
        assert!(RuleMatcher::matches(&rule, &cmd1));

        let cmd2 = ParsedCommand::parse("git remote add origin").unwrap();
        assert!(RuleMatcher::matches(&rule, &cmd2));

        let cmd3 = ParsedCommand::parse("git status").unwrap();
        assert!(!RuleMatcher::matches(&rule, &cmd3));
    }

    #[test]
    fn test_subcommand_exact_match() {
        let mut rule = make_rule(Some("git"), vec!["remote"], Action::Allow);
        rule.subcommands_exact = true;

        let cmd1 = ParsedCommand::parse("git remote").unwrap();
        assert!(RuleMatcher::matches(&rule, &cmd1));

        let cmd2 = ParsedCommand::parse("git remote add origin").unwrap();
        assert!(!RuleMatcher::matches(&rule, &cmd2));
    }

    #[test]
    fn test_flags_present() {
        let mut rule = make_rule(Some("git"), vec!["push"], Action::Deny);
        rule.flags_present = vec!["--force".to_string()];

        let cmd1 = ParsedCommand::parse("git push --force").unwrap();
        assert!(RuleMatcher::matches(&rule, &cmd1));

        let cmd2 = ParsedCommand::parse("git push").unwrap();
        assert!(!RuleMatcher::matches(&rule, &cmd2));
    }

    #[test]
    fn test_flags_absent() {
        let mut rule = make_rule(Some("git"), vec!["push"], Action::Allow);
        rule.flags_absent = vec!["--force".to_string(), "-f".to_string()];

        let cmd1 = ParsedCommand::parse("git push").unwrap();
        assert!(RuleMatcher::matches(&rule, &cmd1));

        let cmd2 = ParsedCommand::parse("git push --force").unwrap();
        assert!(!RuleMatcher::matches(&rule, &cmd2));

        let cmd3 = ParsedCommand::parse("git push -f").unwrap();
        assert!(!RuleMatcher::matches(&rule, &cmd3));
    }

    #[test]
    fn test_args_regex() {
        let mut rule = make_rule(Some("rm"), vec![], Action::Deny);
        rule.args_regex = Some(r"/\*".to_string());

        let cmd1 = ParsedCommand::parse("rm -rf /*").unwrap();
        assert!(RuleMatcher::matches(&rule, &cmd1));

        let cmd2 = ParsedCommand::parse("rm foo.txt").unwrap();
        assert!(!RuleMatcher::matches(&rule, &cmd2));
    }
}
