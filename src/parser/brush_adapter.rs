//! Adapter module to convert brush-parser AST to ParsedCommand
//!
//! This module provides functionality to parse shell commands using brush-parser
//! and convert the resulting AST into Vec<ParsedCommand> for rule evaluation.

use std::collections::HashMap;

use anyhow::{bail, Result};
use brush_parser::{ast, parse_tokens, tokenize_str, unquote_str, ParserOptions, SourceInfo};

use super::{command::ParsedCommand, semantic::SemanticAnalyzer};

/// Parse a command string using brush-parser and return all commands found.
///
/// This extracts ALL commands from pipelines, chains (&&/||), and nested structures,
/// not just the first command. This is a security-critical design decision to prevent
/// bypass via: `allowed-cmd | blocked-cmd` or `safe-cmd && dangerous-cmd`
pub fn parse_with_brush(input: &str) -> Result<Vec<ParsedCommand>> {
    // Tokenize the input
    let tokens = tokenize_str(input).map_err(|e| anyhow::anyhow!("Tokenizer error: {:?}", e))?;

    // Parse tokens into AST
    let options = ParserOptions::default();
    let source_info = SourceInfo::default();
    let program = parse_tokens(&tokens, &options, &source_info)
        .map_err(|e| anyhow::anyhow!("Parse error: {:?}", e))?;

    let mut results = Vec::new();
    let ctx = ExtractionContext::new(input);

    // Walk AST: Program contains complete_commands (which are CompoundLists)
    for complete_command in &program.complete_commands {
        extract_from_compound_list(complete_command, &ctx, &mut results)?;
    }

    // If we got nothing but input wasn't empty, that's an error
    if results.is_empty() && !input.trim().is_empty() {
        bail!("No commands found in input");
    }

    Ok(results)
}

/// Context for command extraction, carrying the original input
struct ExtractionContext<'a> {
    input: &'a str,
}

impl<'a> ExtractionContext<'a> {
    fn new(input: &'a str) -> Self {
        Self { input }
    }
}

/// Extract commands from a CompoundList (Vec<CompoundListItem>)
fn extract_from_compound_list(
    compound_list: &ast::CompoundList,
    ctx: &ExtractionContext,
    results: &mut Vec<ParsedCommand>,
) -> Result<()> {
    // CompoundList is a tuple struct containing Vec<CompoundListItem>
    for item in &compound_list.0 {
        // CompoundListItem is (AndOrList, SeparatorOperator)
        extract_from_and_or_list(&item.0, ctx, results)?;
    }
    Ok(())
}

/// Extract commands from an and-or list (commands joined by && or ||)
fn extract_from_and_or_list(
    and_or: &ast::AndOrList,
    ctx: &ExtractionContext,
    results: &mut Vec<ParsedCommand>,
) -> Result<()> {
    // First pipeline
    extract_from_pipeline(&and_or.first, ctx, results)?;

    // Additional pipelines (joined by && or ||)
    for item in &and_or.additional {
        let pipeline = match item {
            ast::AndOr::And(p) | ast::AndOr::Or(p) => p,
        };
        extract_from_pipeline(pipeline, ctx, results)?;
    }

    Ok(())
}

/// Extract commands from a pipeline
fn extract_from_pipeline(
    pipeline: &ast::Pipeline,
    ctx: &ExtractionContext,
    results: &mut Vec<ParsedCommand>,
) -> Result<()> {
    let is_piped = pipeline.seq.len() > 1;

    for command in &pipeline.seq {
        extract_from_command(command, ctx, is_piped, results)?;
    }

    Ok(())
}

/// Extract commands from a Command enum
fn extract_from_command(
    cmd: &ast::Command,
    ctx: &ExtractionContext,
    is_piped: bool,
    results: &mut Vec<ParsedCommand>,
) -> Result<()> {
    match cmd {
        ast::Command::Simple(simple) => {
            if let Some(parsed) = extract_simple_command(simple, ctx, is_piped)? {
                results.push(parsed);
            }
        }
        ast::Command::Compound(compound, _redirects) => {
            extract_from_compound_command(compound, ctx, results)?;
        }
        ast::Command::ExtendedTest(_test_expr) => {
            // Extended test expressions [[ ... ]] - these don't execute commands
            // but we could potentially analyze them in the future
        }
        ast::Command::Function(_func_def) => {
            // Function definitions don't execute immediately, skip
        }
    }
    Ok(())
}

/// Extract commands from compound commands (if/for/while/case/subshell/brace)
fn extract_from_compound_command(
    compound: &ast::CompoundCommand,
    ctx: &ExtractionContext,
    results: &mut Vec<ParsedCommand>,
) -> Result<()> {
    match compound {
        ast::CompoundCommand::Subshell(subshell) => {
            // Recursively extract from subshell
            extract_from_compound_list(&subshell.list, ctx, results)?;
        }
        ast::CompoundCommand::BraceGroup(brace) => {
            extract_from_compound_list(&brace.list, ctx, results)?;
        }
        ast::CompoundCommand::ForClause(for_clause) => {
            // for_clause.body is DoGroupCommand which has list: CompoundList
            extract_from_compound_list(&for_clause.body.list, ctx, results)?;
        }
        ast::CompoundCommand::CaseClause(case_clause) => {
            // Extract commands from each case item
            for item in &case_clause.cases {
                if let Some(cmd) = &item.cmd {
                    extract_from_compound_list(cmd, ctx, results)?;
                }
            }
        }
        ast::CompoundCommand::IfClause(if_clause) => {
            // Extract from condition and body
            extract_from_compound_list(&if_clause.condition, ctx, results)?;
            extract_from_compound_list(&if_clause.then, ctx, results)?;

            // Extract from else clauses
            if let Some(elses) = &if_clause.elses {
                for else_clause in elses {
                    if let Some(condition) = &else_clause.condition {
                        extract_from_compound_list(condition, ctx, results)?;
                    }
                    extract_from_compound_list(&else_clause.body, ctx, results)?;
                }
            }
        }
        ast::CompoundCommand::WhileClause(while_clause) => {
            // WhileOrUntilClauseCommand is a tuple struct (CompoundList, DoGroupCommand, TokenLocation)
            extract_from_compound_list(&while_clause.0, ctx, results)?;
            extract_from_compound_list(&while_clause.1.list, ctx, results)?;
        }
        ast::CompoundCommand::UntilClause(until_clause) => {
            extract_from_compound_list(&until_clause.0, ctx, results)?;
            extract_from_compound_list(&until_clause.1.list, ctx, results)?;
        }
        ast::CompoundCommand::ArithmeticForClause(arith_for) => {
            extract_from_compound_list(&arith_for.body.list, ctx, results)?;
        }
        ast::CompoundCommand::Arithmetic(_) => {
            // Arithmetic commands don't execute other commands
        }
    }
    Ok(())
}

/// Extract a simple command into ParsedCommand
fn extract_simple_command(
    cmd: &ast::SimpleCommand,
    ctx: &ExtractionContext,
    is_piped: bool,
) -> Result<Option<ParsedCommand>> {
    let mut env_vars: HashMap<String, String> = HashMap::new();
    let mut has_redirect = false;
    let mut words: Vec<String> = Vec::new();

    // Process prefix (assignments and redirects before command)
    if let Some(prefix) = &cmd.prefix {
        // CommandPrefix is a tuple struct containing Vec<CommandPrefixOrSuffixItem>
        for item in &prefix.0 {
            match item {
                ast::CommandPrefixOrSuffixItem::AssignmentWord(assignment, _word) => {
                    let name = assignment_name_to_string(&assignment.name);
                    let value = assignment_value_to_string(&assignment.value);
                    env_vars.insert(name, value);
                }
                ast::CommandPrefixOrSuffixItem::IoRedirect(_) => {
                    has_redirect = true;
                }
                ast::CommandPrefixOrSuffixItem::Word(word) => {
                    words.push(unquote_word(&word.value));
                }
                ast::CommandPrefixOrSuffixItem::ProcessSubstitution(_, _) => {
                    // Process substitutions are like redirects
                    has_redirect = true;
                }
            }
        }
    }

    // Extract the command word (first word)
    if let Some(word) = &cmd.word_or_name {
        words.push(unquote_word(&word.value));
    }

    // Process suffix (args and redirects after command)
    if let Some(suffix) = &cmd.suffix {
        // CommandSuffix is a tuple struct containing Vec<CommandPrefixOrSuffixItem>
        for item in &suffix.0 {
            match item {
                ast::CommandPrefixOrSuffixItem::IoRedirect(_) => {
                    has_redirect = true;
                }
                ast::CommandPrefixOrSuffixItem::Word(word) => {
                    words.push(unquote_word(&word.value));
                }
                ast::CommandPrefixOrSuffixItem::AssignmentWord(assignment, _word) => {
                    // In suffix position, this is actually an argument that looks like an assignment
                    // e.g., `curl VAR=value` - VAR=value is an argument, not an env var
                    let name = assignment_name_to_string(&assignment.name);
                    let value = assignment_value_to_string(&assignment.value);
                    words.push(format!("{}={}", name, value));
                }
                ast::CommandPrefixOrSuffixItem::ProcessSubstitution(_, _) => {
                    has_redirect = true;
                }
            }
        }
    }

    // If no program (just assignments), return None
    if words.is_empty() {
        return Ok(None);
    }

    // Detect expansion and substitution in all words
    let has_expansion = words.iter().any(|w| contains_expansion(w));
    let has_substitution = words.iter().any(|w| contains_substitution(w));

    // Use semantic analyzer
    let program = words[0].clone();
    let remaining: Vec<String> = words[1..].to_vec();

    let analyzer = SemanticAnalyzer::new();
    let (subcommands, flags, args) = analyzer.analyze(&program, &remaining);

    Ok(Some(ParsedCommand {
        raw: ctx.input.to_string(),
        program,
        subcommands,
        args,
        flags,
        is_piped,
        has_redirect,
        env_vars,
        has_expansion,
        has_substitution,
    }))
}

/// Unquote a word value using brush-parser's unquote_str
fn unquote_word(value: &str) -> String {
    unquote_str(value)
}

/// Convert AssignmentName to String
fn assignment_name_to_string(name: &ast::AssignmentName) -> String {
    match name {
        ast::AssignmentName::VariableName(s) => s.clone(),
        ast::AssignmentName::ArrayElementName(name, index) => format!("{}[{}]", name, index),
    }
}

/// Convert AssignmentValue to String
fn assignment_value_to_string(value: &ast::AssignmentValue) -> String {
    match value {
        ast::AssignmentValue::Scalar(word) => word.value.clone(),
        ast::AssignmentValue::Array(elements) => {
            let parts: Vec<String> = elements
                .iter()
                .map(|(key, val)| {
                    if let Some(k) = key {
                        format!("[{}]={}", k.value, val.value)
                    } else {
                        val.value.clone()
                    }
                })
                .collect();
            format!("({})", parts.join(" "))
        }
    }
}

/// Check if a string contains parameter expansion ($VAR but not $(...))
fn contains_expansion(s: &str) -> bool {
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$' {
            if let Some(&next) = chars.peek() {
                // $( is command substitution, not parameter expansion
                if next != '(' && next != '`' {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a string contains command substitution ($(...) or backticks)
fn contains_substitution(s: &str) -> bool {
    s.contains("$(") || s.contains('`')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let results = parse_with_brush("ls -la").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].program, "ls");
        assert!(results[0].flags.contains("-l"));
        assert!(results[0].flags.contains("-a"));
    }

    #[test]
    fn test_pipeline() {
        let results = parse_with_brush("ls | grep foo | wc -l").unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].program, "ls");
        assert_eq!(results[1].program, "grep");
        assert_eq!(results[2].program, "wc");
        assert!(results[0].is_piped);
        assert!(results[1].is_piped);
        assert!(results[2].is_piped);
    }

    #[test]
    fn test_and_chain() {
        let results = parse_with_brush("cd /tmp && ls -la").unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].program, "cd");
        assert_eq!(results[1].program, "ls");
    }

    #[test]
    fn test_or_chain() {
        let results = parse_with_brush("make build || echo failed").unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].program, "make");
        assert_eq!(results[1].program, "echo");
    }

    #[test]
    fn test_env_vars() {
        let results = parse_with_brush("NODE_ENV=production npm start").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].program, "npm");
        assert_eq!(
            results[0].env_vars.get("NODE_ENV"),
            Some(&"production".to_string())
        );
    }

    #[test]
    fn test_redirect() {
        let results = parse_with_brush("echo hello > file.txt").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].program, "echo");
        assert!(results[0].has_redirect);
    }

    #[test]
    fn test_quoted_string() {
        let results = parse_with_brush(r#"echo "hello world""#).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].program, "echo");
        assert!(results[0].args.contains(&"hello world".to_string()));
    }

    #[test]
    fn test_expansion_detection() {
        let results = parse_with_brush("echo $HOME").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].has_expansion);
        assert!(!results[0].has_substitution);
    }

    #[test]
    fn test_substitution_detection() {
        let results = parse_with_brush("echo $(date)").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].has_substitution);
    }

    #[test]
    fn test_git_status() {
        let results = parse_with_brush("git status").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].program, "git");
        assert_eq!(results[0].subcommands, vec!["status"]);
    }

    #[test]
    fn test_git_remote_add() {
        let results = parse_with_brush("git remote add origin https://github.com/foo/bar").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].program, "git");
        assert_eq!(results[0].subcommands, vec!["remote", "add"]);
    }

    #[test]
    fn test_single_command_not_piped() {
        let results = parse_with_brush("ls -la").unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results[0].is_piped);
    }

    #[test]
    fn test_subshell() {
        let results = parse_with_brush("(cd /tmp && ls)").unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].program, "cd");
        assert_eq!(results[1].program, "ls");
    }

    #[test]
    fn test_complex_chain() {
        let results = parse_with_brush("cmd1 && cmd2 | cmd3 || cmd4").unwrap();
        assert_eq!(results.len(), 4);
        assert_eq!(results[0].program, "cmd1");
        assert_eq!(results[1].program, "cmd2");
        assert_eq!(results[2].program, "cmd3");
        assert_eq!(results[3].program, "cmd4");
    }
}
