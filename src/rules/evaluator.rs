use super::matcher::{MatchInfo, MatchReason, RuleMatcher};
use crate::{
    config::{Action, Config, Rule},
    parser::{CapabilitySource, ParsedCommand},
};

/// The decision made about a command
#[derive(Debug, Clone)]
pub enum Decision {
    Allow,
    Deny {
        message: String,
        match_info: Option<MatchInfo>,
    },
    Prompt {
        message: String,
        match_info: Option<MatchInfo>,
    },
}

impl PartialEq for Decision {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Decision::Allow, Decision::Allow) => true,
            (Decision::Deny { message: m1, .. }, Decision::Deny { message: m2, .. }) => m1 == m2,
            (Decision::Prompt { message: m1, .. }, Decision::Prompt { message: m2, .. }) => {
                m1 == m2
            }
            _ => false,
        }
    }
}

impl Eq for Decision {}

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
        let capability_policy = self.evaluate_capability_policies(command);

        if let Some((Action::Deny, capability, source)) = capability_policy.as_ref() {
            let match_info =
                Self::match_info_from_capability_source(command, capability, source.as_ref());
            return (
                Decision::Deny {
                    message: format!("Blocked by global capability policy: {}", capability),
                    match_info,
                },
                None,
            );
        }

        // Then, check inline rules from config
        for rule in &self.config.rules {
            if let Some(match_info) = RuleMatcher::matches_with_info(rule, command) {
                let decision = Self::make_decision(rule, Some(match_info));

                if let Some((Action::Prompt, capability, source)) = capability_policy.as_ref() {
                    if matches!(decision, Decision::Allow) {
                        let match_info = Self::match_info_from_capability_source(
                            command,
                            capability,
                            source.as_ref(),
                        );
                        return (
                            Decision::Prompt {
                                message: format!(
                                    "Requires confirmation by global capability policy: {}",
                                    capability
                                ),
                                match_info,
                            },
                            Some(rule.clone()),
                        );
                    }
                }

                return (decision, Some(rule.clone()));
            }
        }

        if let Some((Action::Prompt, capability, source)) = capability_policy.as_ref() {
            let match_info =
                Self::match_info_from_capability_source(command, capability, source.as_ref());
            return (
                Decision::Prompt {
                    message: format!(
                        "Requires confirmation by global capability policy: {}",
                        capability
                    ),
                    match_info,
                },
                None,
            );
        }

        if let Some((Action::Allow, _, _)) = capability_policy.as_ref() {
            return (Decision::Allow, None);
        }

        // Finally, use default action
        let decision = match self.config.settings.default_action {
            Action::Allow => Decision::Allow,
            Action::Deny => Decision::Deny {
                message: "Blocked by default policy".to_string(),
                match_info: None,
            },
            Action::Prompt => Decision::Prompt {
                message: "No matching rule found".to_string(),
                match_info: None,
            },
        };

        (decision, None)
    }

    fn evaluate_capability_policies(
        &self,
        command: &ParsedCommand,
    ) -> Option<(Action, String, Option<CapabilitySource>)> {
        let mut matches: Vec<(&String, &Action)> = command
            .capabilities
            .iter()
            .filter_map(|capability| {
                self.config
                    .capabilities
                    .get(capability)
                    .map(|action| (capability, action))
            })
            .collect();

        if matches.is_empty() {
            return None;
        }

        matches.sort_by(|(a, _), (b, _)| a.cmp(b));

        if let Some((capability, _)) = matches
            .iter()
            .find(|(_, action)| matches!(action, Action::Deny))
        {
            let source = command.capability_sources.get(*capability).cloned();
            return Some((Action::Deny, (*capability).clone(), source));
        }

        if let Some((capability, _)) = matches
            .iter()
            .find(|(_, action)| matches!(action, Action::Prompt))
        {
            let source = command.capability_sources.get(*capability).cloned();
            return Some((Action::Prompt, (*capability).clone(), source));
        }

        let capability = matches[0].0.clone();
        let source = command.capability_sources.get(&capability).cloned();
        Some((Action::Allow, capability, source))
    }

    /// Create a MatchInfo from a capability source
    fn match_info_from_capability_source(
        command: &ParsedCommand,
        capability: &str,
        source: Option<&CapabilitySource>,
    ) -> Option<MatchInfo> {
        match source {
            Some(CapabilitySource::Flag(flag)) => {
                // Find the flag position in the raw command
                command.raw.find(flag.as_str()).map(|pos| MatchInfo {
                    reason: MatchReason::FlagPresent(flag.clone()),
                    span_start: pos,
                    span_end: pos + flag.len(),
                })
            }
            Some(CapabilitySource::Subcommand(subcmd)) => {
                // Find the subcommand position in the raw command
                command.raw.find(subcmd.as_str()).map(|pos| MatchInfo {
                    reason: MatchReason::Subcommands(vec![subcmd.clone()]),
                    span_start: pos,
                    span_end: pos + subcmd.len(),
                })
            }
            Some(CapabilitySource::Program) | None => {
                // Point to the program name
                command.raw.find(&command.program).map(|pos| MatchInfo {
                    reason: MatchReason::Program(capability.to_string()),
                    span_start: pos,
                    span_end: pos + command.program.len(),
                })
            }
        }
    }

    fn make_decision(rule: &Rule, match_info: Option<MatchInfo>) -> Decision {
        match rule.action {
            Action::Allow => Decision::Allow,
            Action::Deny => Decision::Deny {
                message: rule
                    .message
                    .clone()
                    .unwrap_or_else(|| "Blocked by rule".to_string()),
                match_info,
            },
            Action::Prompt => Decision::Prompt {
                message: rule
                    .message
                    .clone()
                    .unwrap_or_else(|| "Requires confirmation".to_string()),
                match_info,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::config::Settings;

    fn make_config_with_rules(rules: Vec<Rule>) -> Config {
        Config {
            settings: Settings::default(),
            capabilities: HashMap::new(),
            rules,
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
                message: "Push not allowed".to_string(),
                match_info: None,
            }
        );
    }

    #[test]
    fn test_default_action() {
        let config = Config {
            settings: Settings {
                default_action: Action::Deny,
                log_decisions: false,
            },
            capabilities: HashMap::new(),
            rules: vec![],
        };

        let cmds = ParsedCommand::parse_all("some-unknown-command").unwrap();
        let evaluator = Evaluator::new(&config);
        let decision = evaluator.evaluate_all(&cmds);

        assert_eq!(
            decision,
            Decision::Deny {
                message: "Blocked by default policy".to_string(),
                match_info: None,
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
                message: "rm blocked".to_string(),
                match_info: None,
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
                message: "dangerous blocked".to_string(),
                match_info: None,
            }
        );
    }

    #[test]
    fn test_global_capability_deny() {
        let mut config = make_config_with_rules(vec![]);
        config.settings.default_action = Action::Allow;
        config
            .capabilities
            .insert("git.force_push".to_string(), Action::Deny);

        let cmds = ParsedCommand::parse_all("git push --force origin main").unwrap();
        let evaluator = Evaluator::new(&config);
        let decision = evaluator.evaluate_all(&cmds);

        assert_eq!(
            decision,
            Decision::Deny {
                message: "Blocked by global capability policy: git.force_push".to_string(),
                match_info: None,
            }
        );
    }

    #[test]
    fn test_global_capability_prompt() {
        let mut config = make_config_with_rules(vec![]);
        config.settings.default_action = Action::Allow;
        config
            .capabilities
            .insert("git.push".to_string(), Action::Prompt);

        let cmds = ParsedCommand::parse_all("git push origin main").unwrap();
        let evaluator = Evaluator::new(&config);
        let decision = evaluator.evaluate_all(&cmds);

        assert_eq!(
            decision,
            Decision::Prompt {
                message: "Requires confirmation by global capability policy: git.push".to_string(),
                match_info: None,
            }
        );
    }

    #[test]
    fn test_global_capability_allow_overrides_default_deny() {
        let mut config = make_config_with_rules(vec![]);
        config.settings.default_action = Action::Deny;
        config
            .capabilities
            .insert("git.read-local".to_string(), Action::Allow);

        let cmds = ParsedCommand::parse_all("git status").unwrap();
        let evaluator = Evaluator::new(&config);
        let decision = evaluator.evaluate_all(&cmds);

        assert_eq!(decision, Decision::Allow);
    }

    #[test]
    fn test_global_capability_deny_precedence_over_allow() {
        let mut config = make_config_with_rules(vec![]);
        config.settings.default_action = Action::Allow;
        config
            .capabilities
            .insert("git.push".to_string(), Action::Allow);
        config
            .capabilities
            .insert("git.force_push".to_string(), Action::Deny);

        let cmds = ParsedCommand::parse_all("git push --force origin main").unwrap();
        let evaluator = Evaluator::new(&config);
        let decision = evaluator.evaluate_all(&cmds);

        assert_eq!(
            decision,
            Decision::Deny {
                message: "Blocked by global capability policy: git.force_push".to_string(),
                match_info: None,
            }
        );
    }
}
