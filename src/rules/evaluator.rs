use super::matcher::RuleMatcher;
use crate::{
    config::{Action, Config, Rule},
    parser::ParsedCommand,
};

/// The decision made about a command
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny { message: String },
    Prompt { message: String },
}

/// Evaluates commands against rules
pub struct Evaluator<'a> {
    config: &'a Config,
}

impl<'a> Evaluator<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self { config }
    }

    /// Evaluate all commands and return the strictest decision.
    ///
    /// - If any command is Deny, the overall result is Deny
    /// - If any command is Prompt (and none is Deny), the overall result is Prompt
    /// - Only if all commands are Allow, the overall result is Allow
    ///
    /// This is the recommended method for security evaluation as it prevents bypass
    /// via pipelines or command chains.
    pub fn evaluate_all(&self, commands: &[ParsedCommand]) -> Decision {
        self.evaluate_all_with_trace(commands).0
    }

    /// Evaluate all commands and return the strictest decision along with the matched rule.
    pub fn evaluate_all_with_trace(&self, commands: &[ParsedCommand]) -> (Decision, Option<Rule>) {
        let mut strictest_decision = Decision::Allow;
        let mut matched_rule: Option<Rule> = None;

        for command in commands {
            let (decision, rule) = self.evaluate_single_with_trace(command);

            // Update to strictest decision: Deny > Prompt > Allow
            match (&strictest_decision, &decision) {
                (Decision::Allow, Decision::Deny { .. })
                | (Decision::Allow, Decision::Prompt { .. })
                | (Decision::Prompt { .. }, Decision::Deny { .. }) => {
                    strictest_decision = decision;
                    matched_rule = rule;
                }
                _ => {}
            }

            // Short-circuit on Deny - can't get stricter
            if matches!(strictest_decision, Decision::Deny { .. }) {
                break;
            }
        }

        (strictest_decision, matched_rule)
    }

    /// Evaluate a single command and return the decision.
    ///
    /// **Note:** For security evaluation with pipelines or chains, use `evaluate_all()`.
    pub fn evaluate(&self, command: &ParsedCommand) -> Decision {
        self.evaluate_single_with_trace(command).0
    }

    /// Evaluate a single command and return both the decision and the matched rule (if any).
    ///
    /// **Note:** For security evaluation with pipelines or chains, use `evaluate_all_with_trace()`.
    pub fn evaluate_with_trace(&self, command: &ParsedCommand) -> (Decision, Option<Rule>) {
        self.evaluate_single_with_trace(command)
    }

    /// Internal method to evaluate a single command.
    fn evaluate_single_with_trace(&self, command: &ParsedCommand) -> (Decision, Option<Rule>) {
        // First, check custom rules from config (highest priority)
        for rule in &self.config.rules {
            if RuleMatcher::matches(rule, command) {
                return (Self::make_decision(rule), Some(rule.clone()));
            }
        }

        // Then, check profile rules (in order of profiles)
        for profile in &self.config.loaded_profiles {
            for rule in &profile.rules {
                if RuleMatcher::matches(rule, command) {
                    return (Self::make_decision(rule), Some(rule.clone()));
                }
            }
        }

        // Finally, use default action
        let decision = match self.config.settings.default_action {
            Action::Allow => Decision::Allow,
            Action::Deny => Decision::Deny {
                message: "Blocked by default policy".to_string(),
            },
            Action::Prompt => Decision::Prompt {
                message: "No matching rule found".to_string(),
            },
        };

        (decision, None)
    }

    fn make_decision(rule: &Rule) -> Decision {
        match rule.action {
            Action::Allow => Decision::Allow,
            Action::Deny => Decision::Deny {
                message: rule
                    .message
                    .clone()
                    .unwrap_or_else(|| "Blocked by rule".to_string()),
            },
            Action::Prompt => Decision::Prompt {
                message: rule
                    .message
                    .clone()
                    .unwrap_or_else(|| "Requires confirmation".to_string()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Profile, ProfileMetadata, ProfilesConfig, Settings};

    fn make_config_with_rules(rules: Vec<Rule>) -> Config {
        Config {
            settings: Settings::default(),
            profiles: ProfilesConfig {
                builtins: vec![],
                custom: vec![],
            },
            rules,
            loaded_profiles: vec![],
            available_profiles: vec![],
        }
    }

    #[test]
    fn test_allow_rule() {
        let config = make_config_with_rules(vec![Rule {
            program: Some("git".to_string()),
            subcommands: vec!["status".to_string()],
            subcommands_exact: false,
            args_match: None,
            args_regex: None,
            flags_present: vec![],
            flags_absent: vec![],
            working_dir: None,
            action: Action::Allow,
            message: None,
        }]);

        let cmds = ParsedCommand::parse_all("git status").unwrap();
        let evaluator = Evaluator::new(&config);
        let decision = evaluator.evaluate_all(&cmds);

        assert_eq!(decision, Decision::Allow);
    }

    #[test]
    fn test_deny_rule() {
        let config = make_config_with_rules(vec![Rule {
            program: Some("git".to_string()),
            subcommands: vec!["push".to_string()],
            subcommands_exact: false,
            args_match: None,
            args_regex: None,
            flags_present: vec![],
            flags_absent: vec![],
            working_dir: None,
            action: Action::Deny,
            message: Some("Push not allowed".to_string()),
        }]);

        let cmds = ParsedCommand::parse_all("git push origin main").unwrap();
        let evaluator = Evaluator::new(&config);
        let decision = evaluator.evaluate_all(&cmds);

        assert_eq!(
            decision,
            Decision::Deny {
                message: "Push not allowed".to_string()
            }
        );
    }

    #[test]
    fn test_profile_rules() {
        let config = Config {
            settings: Settings::default(),
            profiles: ProfilesConfig {
                builtins: vec!["test".to_string()],
                custom: vec![],
            },
            rules: vec![],
            loaded_profiles: vec![Profile {
                profile: ProfileMetadata {
                    name: "test".to_string(),
                    description: None,
                },
                rules: vec![Rule {
                    program: Some("rm".to_string()),
                    subcommands: vec![],
                    subcommands_exact: false,
                    args_match: None,
                    args_regex: None,
                    flags_present: vec!["-r".to_string()],
                    flags_absent: vec![],
                    working_dir: None,
                    action: Action::Deny,
                    message: Some("Recursive delete blocked".to_string()),
                }],
            }],
            available_profiles: vec![],
        };

        let cmds = ParsedCommand::parse_all("rm -rf /tmp/foo").unwrap();
        let evaluator = Evaluator::new(&config);
        let decision = evaluator.evaluate_all(&cmds);

        assert_eq!(
            decision,
            Decision::Deny {
                message: "Recursive delete blocked".to_string()
            }
        );
    }

    #[test]
    fn test_custom_rules_override_profiles() {
        let config = Config {
            settings: Settings::default(),
            profiles: ProfilesConfig {
                builtins: vec![],
                custom: vec![],
            },
            rules: vec![Rule {
                program: Some("git".to_string()),
                subcommands: vec!["push".to_string()],
                subcommands_exact: false,
                args_match: None,
                args_regex: None,
                flags_present: vec![],
                flags_absent: vec![],
                working_dir: None,
                action: Action::Allow,
                message: None,
            }],
            loaded_profiles: vec![Profile {
                profile: ProfileMetadata {
                    name: "test".to_string(),
                    description: None,
                },
                rules: vec![Rule {
                    program: Some("git".to_string()),
                    subcommands: vec!["push".to_string()],
                    subcommands_exact: false,
                    args_match: None,
                    args_regex: None,
                    flags_present: vec![],
                    flags_absent: vec![],
                    working_dir: None,
                    action: Action::Deny,
                    message: Some("Blocked by profile".to_string()),
                }],
            }],
            available_profiles: vec![],
        };

        let cmds = ParsedCommand::parse_all("git push").unwrap();
        let evaluator = Evaluator::new(&config);
        let decision = evaluator.evaluate_all(&cmds);

        // Custom rule should take precedence
        assert_eq!(decision, Decision::Allow);
    }

    #[test]
    fn test_default_action() {
        let config = Config {
            settings: Settings {
                default_action: Action::Deny,
                log_decisions: false,
            },
            profiles: ProfilesConfig {
                builtins: vec![],
                custom: vec![],
            },
            rules: vec![],
            loaded_profiles: vec![],
            available_profiles: vec![],
        };

        let cmds = ParsedCommand::parse_all("some-unknown-command").unwrap();
        let evaluator = Evaluator::new(&config);
        let decision = evaluator.evaluate_all(&cmds);

        assert_eq!(
            decision,
            Decision::Deny {
                message: "Blocked by default policy".to_string()
            }
        );
    }

    #[test]
    fn test_pipeline_deny_second_command() {
        // Test that a denied command in a pipeline causes overall deny
        let config = make_config_with_rules(vec![
            Rule {
                program: Some("ls".to_string()),
                subcommands: vec![],
                subcommands_exact: false,
                args_match: None,
                args_regex: None,
                flags_present: vec![],
                flags_absent: vec![],
                working_dir: None,
                action: Action::Allow,
                message: None,
            },
            Rule {
                program: Some("rm".to_string()),
                subcommands: vec![],
                subcommands_exact: false,
                args_match: None,
                args_regex: None,
                flags_present: vec![],
                flags_absent: vec![],
                working_dir: None,
                action: Action::Deny,
                message: Some("rm blocked".to_string()),
            },
        ]);

        // "ls" is allowed, but "rm" is denied - overall should be deny
        // Using a direct pipeline where rm is actually a command
        let cmds = ParsedCommand::parse_all("ls | rm -rf").unwrap();
        let evaluator = Evaluator::new(&config);
        let decision = evaluator.evaluate_all(&cmds);

        assert_eq!(
            decision,
            Decision::Deny {
                message: "rm blocked".to_string()
            }
        );
    }

    #[test]
    fn test_chain_deny_blocks_all() {
        // Test that deny in && chain causes overall deny
        let config = make_config_with_rules(vec![Rule {
            program: Some("dangerous".to_string()),
            subcommands: vec![],
            subcommands_exact: false,
            args_match: None,
            args_regex: None,
            flags_present: vec![],
            flags_absent: vec![],
            working_dir: None,
            action: Action::Deny,
            message: Some("dangerous blocked".to_string()),
        }]);

        let cmds = ParsedCommand::parse_all("safe-cmd && dangerous").unwrap();
        let evaluator = Evaluator::new(&config);
        let decision = evaluator.evaluate_all(&cmds);

        assert_eq!(
            decision,
            Decision::Deny {
                message: "dangerous blocked".to_string()
            }
        );
    }
}
