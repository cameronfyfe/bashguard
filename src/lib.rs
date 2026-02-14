#![doc = include_str!("../README.md")]

pub mod cli;
pub mod cmd_map;
pub mod config;
pub mod logger;
pub mod parser;
pub mod rules;

pub use config::{Config, Settings};
pub use logger::SessionLogger;
pub use parser::ParsedCommand;
pub use rules::{Decision, Evaluator, MatchInfo, MatchReason};
