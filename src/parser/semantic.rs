use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
};

use crate::{
    cmd_map::{CmdCapability, CmdMap},
    parser::command::CapabilitySource,
};

/// Result of semantic analysis with capability information
#[allow(dead_code)]
type AnalysisResult = (
    Vec<String>,                       // subcommands
    HashSet<String>,                   // flags
    Vec<String>,                       // args
    HashSet<CmdCapability>,            // capabilities
    HashMap<String, CapabilitySource>, // capability_sources
);

/// Known programs and their subcommand patterns
#[derive(Debug)]
struct ProgramInfo {
    /// Maximum depth of subcommands (e.g., git remote add = 2)
    max_subcommand_depth: usize,
    /// Known subcommands for this program
    known_subcommands: HashSet<String>,
    /// Optional command map tree used for capability tagging
    cmd_map: Option<CmdMap>,
}

/// Semantic analyzer that extracts structured information from commands
pub struct SemanticAnalyzer {
    programs: HashMap<&'static str, ProgramInfo>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut programs = HashMap::new();

        if let Some(cmd_map) = Self::load_cmd_map("git") {
            programs.insert("git", Self::program_info_from_cmd_map(cmd_map));
        }

        if let Some(cmd_map) = Self::load_cmd_map("docker") {
            programs.insert("docker", Self::program_info_from_cmd_map(cmd_map));
        }

        if let Some(cmd_map) = Self::load_cmd_map("grep") {
            programs.insert("grep", Self::program_info_from_cmd_map(cmd_map));
        }

        if let Some(cmd_map) = Self::load_cmd_map("cat") {
            programs.insert("cat", Self::program_info_from_cmd_map(cmd_map));
        }

        if let Some(cmd_map) = Self::load_cmd_map("less") {
            programs.insert("less", Self::program_info_from_cmd_map(cmd_map));
        }

        if let Some(cmd_map) = Self::load_cmd_map("head") {
            programs.insert("head", Self::program_info_from_cmd_map(cmd_map));
        }

        if let Some(cmd_map) = Self::load_cmd_map("tail") {
            programs.insert("tail", Self::program_info_from_cmd_map(cmd_map));
        }

        // kubectl
        programs.insert(
            "kubectl",
            ProgramInfo {
                max_subcommand_depth: 2,
                known_subcommands: [
                    // Top-level subcommands
                    "alpha",
                    "annotate",
                    "api-resources",
                    "api-versions",
                    "apply",
                    "attach",
                    "auth",
                    "autoscale",
                    "certificate",
                    "cluster-info",
                    "completion",
                    "config",
                    "cordon",
                    "cp",
                    "create",
                    "debug",
                    "delete",
                    "describe",
                    "diff",
                    "drain",
                    "edit",
                    "exec",
                    "explain",
                    "expose",
                    "get",
                    "kustomize",
                    "label",
                    "logs",
                    "options",
                    "patch",
                    "plugin",
                    "port-forward",
                    "proxy",
                    "replace",
                    "rollout",
                    "run",
                    "scale",
                    "set",
                    "taint",
                    "top",
                    "uncordon",
                    "version",
                    "wait",
                    // config subcommands
                    "view",
                    "get-contexts",
                    "current-context",
                    "get-clusters",
                    "get-users",
                    "set-context",
                    "set-cluster",
                    "set-credentials",
                    "use-context",
                    "delete-context",
                    "delete-cluster",
                    "delete-user",
                    "rename-context",
                    // auth subcommands
                    "can-i",
                    "whoami",
                    // rollout subcommands
                    "status",
                    "history",
                    "restart",
                    "undo",
                    "pause",
                    "resume",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
                cmd_map: None,
            },
        );

        // terraform
        programs.insert(
            "terraform",
            ProgramInfo {
                max_subcommand_depth: 2,
                known_subcommands: [
                    // Top-level subcommands
                    "apply",
                    "console",
                    "destroy",
                    "fmt",
                    "force-unlock",
                    "get",
                    "graph",
                    "import",
                    "init",
                    "login",
                    "logout",
                    "metadata",
                    "output",
                    "plan",
                    "providers",
                    "refresh",
                    "show",
                    "state",
                    "taint",
                    "test",
                    "untaint",
                    "validate",
                    "version",
                    "workspace",
                    // state subcommands
                    "list",
                    "mv",
                    "pull",
                    "push",
                    "replace-provider",
                    "rm",
                    // workspace subcommands
                    "delete",
                    "new",
                    "select",
                    // providers subcommands
                    "lock",
                    "mirror",
                    "schema",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
                cmd_map: None,
            },
        );

        // cargo
        programs.insert(
            "cargo",
            ProgramInfo {
                max_subcommand_depth: 1,
                known_subcommands: [
                    "add",
                    "bench",
                    "build",
                    "check",
                    "clean",
                    "clippy",
                    "doc",
                    "fetch",
                    "fix",
                    "fmt",
                    "generate-lockfile",
                    "init",
                    "install",
                    "locate-project",
                    "login",
                    "logout",
                    "metadata",
                    "new",
                    "owner",
                    "package",
                    "pkgid",
                    "publish",
                    "read-manifest",
                    "remove",
                    "report",
                    "run",
                    "rustc",
                    "rustdoc",
                    "search",
                    "test",
                    "tree",
                    "uninstall",
                    "update",
                    "vendor",
                    "verify-project",
                    "version",
                    "yank",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
                cmd_map: None,
            },
        );

        // Azure CLI (az)
        programs.insert(
            "az",
            ProgramInfo {
                max_subcommand_depth: 4, // e.g., az storage account keys list
                known_subcommands: [
                    // Top-level groups
                    "account",
                    "acr",
                    "ad",
                    "advisor",
                    "aks",
                    "apim",
                    "appconfig",
                    "appservice",
                    "backup",
                    "batch",
                    "bicep",
                    "billing",
                    "cdn",
                    "cloud",
                    "cognitiveservices",
                    "config",
                    "configure",
                    "consumption",
                    "container",
                    "cosmosdb",
                    "deployment",
                    "disk",
                    "eventgrid",
                    "eventhubs",
                    "extension",
                    "feature",
                    "functionapp",
                    "group",
                    "hdinsight",
                    "identity",
                    "image",
                    "iot",
                    "keyvault",
                    "lab",
                    "lock",
                    "login",
                    "logout",
                    "logic",
                    "managed-cassandra",
                    "managedapp",
                    "maps",
                    "mariadb",
                    "ml",
                    "monitor",
                    "mysql",
                    "netappfiles",
                    "network",
                    "policy",
                    "postgres",
                    "ppg",
                    "provider",
                    "redis",
                    "relay",
                    "reservations",
                    "resource",
                    "role",
                    "search",
                    "security",
                    "servicebus",
                    "sf",
                    "sig",
                    "signalr",
                    "snapshot",
                    "sql",
                    "ssh",
                    "sshkey",
                    "staticwebapp",
                    "storage",
                    "synapse",
                    "tag",
                    "term",
                    "ts",
                    "version",
                    "vm",
                    "vmss",
                    "webapp",
                    // Common second-level subcommands
                    "server",
                    "db",
                    "database",
                    "container",
                    "blob",
                    "queue",
                    "table",
                    "file",
                    "share",
                    "vnet",
                    "subnet",
                    "nsg",
                    "nic",
                    "lb",
                    "public-ip",
                    "private-endpoint",
                    "application-gateway",
                    "firewall",
                    "dns",
                    "front-door",
                    "traffic-manager",
                    "express-route",
                    "vpn-gateway",
                    "nat",
                    "bastion",
                    "user",
                    "sp",
                    "app",
                    "secret",
                    "key",
                    "certificate",
                    "nodepool",
                    "assignment",
                    "definition",
                    "repository",
                    "rule",
                    "member",
                    "workspace",
                    "activity-log",
                    "log-analytics",
                    "metrics",
                    "diagnostic-settings",
                    "action-group",
                    "alert",
                    "autoscale",
                    "appsettings",
                    "connection-string",
                    "deployment-slot",
                    "keys",
                    "credential",
                    // Common action verbs
                    "list",
                    "show",
                    "create",
                    "delete",
                    "update",
                    "set",
                    "get",
                    "add",
                    "remove",
                    "start",
                    "stop",
                    "restart",
                    "scale",
                    "upgrade",
                    "resize",
                    "exists",
                    "regenerate",
                    "reset",
                    "upload",
                    "download",
                    "copy",
                    "move",
                    "import",
                    "export",
                    "backup",
                    "restore",
                    "build",
                    "query",
                    "invoke",
                    "run",
                    "wait",
                    "tail",
                    "list-defaults",
                    "get-credentials",
                    "get-versions",
                    "get-access-token",
                    "show-connection-string",
                    "list-locations",
                    "list-ip-addresses",
                    "list-sizes",
                    "list-skus",
                    "list-usage",
                    "get-instance-view",
                    "show-tags",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
                cmd_map: None,
            },
        );

        Self { programs }
    }

    /// Analyze a command and extract subcommands, flags, and args
    pub fn analyze(
        &self,
        program: &str,
        remaining: &[String],
    ) -> (Vec<String>, HashSet<String>, Vec<String>) {
        let (subcommands, flags, args, _, _) = self.analyze_with_capabilities(program, remaining);
        (subcommands, flags, args)
    }

    /// Analyze a command and extract subcommands, flags, args, and capability tags.
    pub fn analyze_with_capabilities(&self, program: &str, remaining: &[String]) -> AnalysisResult {
        let mut subcommands = Vec::new();
        let mut flags = HashSet::new();
        let mut args = Vec::new();
        let mut capabilities: HashSet<CmdCapability> = HashSet::new();
        let mut capability_sources: HashMap<String, CapabilitySource> = HashMap::new();

        let program_info = self.programs.get(program);
        let max_depth = program_info.map(|p| p.max_subcommand_depth).unwrap_or(0);
        let known_subcommands = program_info.map(|p| &p.known_subcommands);
        let mut current_cmd_map = program_info.and_then(|p| p.cmd_map.as_ref());
        let mut active_cmd_maps: Vec<&CmdMap> = Vec::new();

        if let Some(cmd_map) = current_cmd_map {
            active_cmd_maps.push(cmd_map);
            for cap in &cmd_map.capabilities {
                capabilities.insert(cap.clone());
                capability_sources.insert(cap.clone(), CapabilitySource::Program);
            }
        }

        let mut in_subcommand_region = true;
        let mut subcommand_depth = 0;

        for word in remaining {
            if word.starts_with('-') {
                // It's a flag
                in_subcommand_region = false;
                Self::parse_flags(word, &mut flags);
            } else if in_subcommand_region && subcommand_depth < max_depth {
                if let Some(next_map) = current_cmd_map
                    .and_then(|cmd_map| cmd_map.subcmds.iter().find(|subcmd| subcmd.cmd == *word))
                {
                    subcommands.push(word.clone());
                    subcommand_depth += 1;
                    for cap in &next_map.capabilities {
                        capabilities.insert(cap.clone());
                        capability_sources
                            .insert(cap.clone(), CapabilitySource::Subcommand(word.clone()));
                    }
                    active_cmd_maps.push(next_map);
                    current_cmd_map = Some(next_map);
                } else if current_cmd_map.is_none()
                    && known_subcommands
                        .map(|sc| sc.contains(word.as_str()))
                        .unwrap_or(false)
                {
                    // Static subcommand catalog (programs not migrated to cmd_maps yet)
                    subcommands.push(word.clone());
                    subcommand_depth += 1;
                } else {
                    // Not a known subcommand, treat as arg
                    in_subcommand_region = false;
                    args.push(word.clone());
                }
            } else {
                // It's an argument
                args.push(word.clone());
            }
        }

        for flag in &flags {
            for cmd_map in &active_cmd_maps {
                if let Some(additions) = cmd_map.flags_add_capabilities.get(flag) {
                    for cap in additions {
                        capabilities.insert(cap.clone());
                        capability_sources
                            .insert(cap.clone(), CapabilitySource::Flag(flag.clone()));
                    }
                }
                if let Some(removals) = cmd_map.flags_remove_capabilities.get(flag) {
                    for capability in removals {
                        capabilities.remove(capability);
                        capability_sources.remove(capability);
                    }
                }
            }
        }

        (subcommands, flags, args, capabilities, capability_sources)
    }

    fn load_cmd_map(program: &str) -> Option<CmdMap> {
        let filename = format!("{program}.toml");
        let mut paths = Vec::new();

        if let Ok(custom_dir) = std::env::var("BASHGUARD_CMD_MAP_DIR") {
            paths.push(PathBuf::from(custom_dir).join(&filename));
        }
        paths.push(PathBuf::from("cmd_maps").join(&filename));
        paths.push(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("cmd_maps")
                .join(&filename),
        );

        for path in paths {
            let Ok(contents) = fs::read_to_string(&path) else {
                continue;
            };
            let Ok(cmd_map) = toml::from_str::<CmdMap>(&contents) else {
                continue;
            };
            if cmd_map.cmd == program {
                return Some(cmd_map);
            }
        }

        // Embedded fallbacks for when cmd_maps directory isn't available
        let embedded = match program {
            "git" => Some(include_str!("../../cmd_maps/git.toml")),
            "docker" => Some(include_str!("../../cmd_maps/docker.toml")),
            "grep" => Some(include_str!("../../cmd_maps/grep.toml")),
            "cat" => Some(include_str!("../../cmd_maps/cat.toml")),
            "less" => Some(include_str!("../../cmd_maps/less.toml")),
            "head" => Some(include_str!("../../cmd_maps/head.toml")),
            "tail" => Some(include_str!("../../cmd_maps/tail.toml")),
            _ => None,
        };

        if let Some(contents) = embedded {
            if let Ok(cmd_map) = toml::from_str::<CmdMap>(contents) {
                if cmd_map.cmd == program {
                    return Some(cmd_map);
                }
            }
        }

        None
    }

    fn program_info_from_cmd_map(cmd_map: CmdMap) -> ProgramInfo {
        let mut known_subcommands = HashSet::new();
        Self::collect_subcommands(&cmd_map, &mut known_subcommands);

        ProgramInfo {
            max_subcommand_depth: Self::max_subcommand_depth(&cmd_map),
            known_subcommands,
            cmd_map: Some(cmd_map),
        }
    }

    fn collect_subcommands(cmd_map: &CmdMap, known_subcommands: &mut HashSet<String>) {
        for subcmd in &cmd_map.subcmds {
            known_subcommands.insert(subcmd.cmd.clone());
            Self::collect_subcommands(subcmd, known_subcommands);
        }
    }

    fn max_subcommand_depth(cmd_map: &CmdMap) -> usize {
        cmd_map
            .subcmds
            .iter()
            .map(|subcmd| 1 + Self::max_subcommand_depth(subcmd))
            .max()
            .unwrap_or(0)
    }

    fn parse_flags(word: &str, flags: &mut HashSet<String>) {
        if word.starts_with("--") {
            // Long flag: --force, --no-verify
            let flag = word.split('=').next().unwrap();
            flags.insert(flag.to_string());
        } else if word.starts_with('-') && word.len() > 1 {
            // Short flags: -f, -rf (combined)
            for c in word[1..].chars() {
                if c.is_alphabetic() {
                    flags.insert(format!("-{}", c));
                }
            }
        }
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_subcommands() {
        let analyzer = SemanticAnalyzer::new();
        let (subcmds, _, _) = analyzer.analyze(
            "git",
            &[
                "remote".to_string(),
                "add".to_string(),
                "origin".to_string(),
            ],
        );
        assert_eq!(subcmds, vec!["remote", "add"]);
    }

    #[test]
    fn test_combined_short_flags() {
        let analyzer = SemanticAnalyzer::new();
        let (_, flags, _) = analyzer.analyze("rm", &["-rf".to_string(), "foo".to_string()]);
        assert!(flags.contains("-r"));
        assert!(flags.contains("-f"));
    }

    #[test]
    fn test_long_flag_with_value() {
        let analyzer = SemanticAnalyzer::new();
        let (_, flags, _) = analyzer.analyze(
            "git",
            &["commit".to_string(), "--message=hello".to_string()],
        );
        assert!(flags.contains("--message"));
    }

    #[test]
    fn test_unknown_program() {
        let analyzer = SemanticAnalyzer::new();
        let (subcmds, flags, args) = analyzer.analyze(
            "myprogram",
            &["foo".to_string(), "-x".to_string(), "bar".to_string()],
        );
        // Unknown program has no subcommand detection
        assert!(subcmds.is_empty());
        assert!(flags.contains("-x"));
        assert_eq!(args, vec!["foo", "bar"]);
    }

    #[test]
    fn test_git_push_force_capability() {
        let analyzer = SemanticAnalyzer::new();
        let (_, _, _, capabilities, capability_sources) =
            analyzer.analyze_with_capabilities("git", &["push".to_string(), "--force".to_string()]);
        assert!(capabilities.contains("git.push"));
        assert!(capabilities.contains("git.force_push"));
        // Verify source tracking
        assert!(matches!(
            capability_sources.get("git.force_push"),
            Some(CapabilitySource::Flag(f)) if f == "--force"
        ));
    }

    #[test]
    fn test_git_remote_add_capability() {
        let analyzer = SemanticAnalyzer::new();
        let (_, _, _, capabilities, _) = analyzer.analyze_with_capabilities(
            "git",
            &[
                "remote".to_string(),
                "add".to_string(),
                "origin".to_string(),
                "https://github.com/foo/bar".to_string(),
            ],
        );
        assert!(capabilities.contains("git.write-local"));
    }
}
