use std::collections::HashSet;

/// Known programs and their subcommand patterns
#[derive(Debug)]
struct ProgramInfo {
    /// Maximum depth of subcommands (e.g., git remote add = 2)
    max_subcommand_depth: usize,
    /// Known subcommands for this program
    known_subcommands: HashSet<&'static str>,
}

/// Semantic analyzer that extracts structured information from commands
pub struct SemanticAnalyzer {
    programs: std::collections::HashMap<&'static str, ProgramInfo>,
}

// TODO: Move subcommand catalogging to dynamic config files that can be updated separately
//
//       Or even better is find OSS project that maintains semantic databases for CLI tools
//       or tools for generating them from man pages or help output.

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut programs = std::collections::HashMap::new();

        // Git has many subcommands, some nested (remote add, remote remove, etc.)
        programs.insert(
            "git",
            ProgramInfo {
                max_subcommand_depth: 2,
                known_subcommands: [
                    // Top-level subcommands
                    "add",
                    "am",
                    "archive",
                    "bisect",
                    "blame",
                    "branch",
                    "bundle",
                    "checkout",
                    "cherry",
                    "cherry-pick",
                    "citool",
                    "clean",
                    "clone",
                    "commit",
                    "config",
                    "describe",
                    "diff",
                    "difftool",
                    "fetch",
                    "format-patch",
                    "gc",
                    "grep",
                    "gui",
                    "help",
                    "init",
                    "log",
                    "merge",
                    "mergetool",
                    "mv",
                    "notes",
                    "pull",
                    "push",
                    "rebase",
                    "reflog",
                    "remote",
                    "reset",
                    "restore",
                    "revert",
                    "rm",
                    "shortlog",
                    "show",
                    "stash",
                    "status",
                    "submodule",
                    "switch",
                    "tag",
                    "worktree",
                    // Nested subcommands (under remote, stash, etc.)
                    "set-url",
                    "get-url",
                    "show-ref",
                    "update-ref",
                    "apply",
                    "drop",
                    "list",
                    "pop",
                    "save",
                    "clear",
                    "prune",
                    "update",
                    "set-head",
                    "rename",
                    "remove",
                ]
                .iter()
                .copied()
                .collect(),
            },
        );

        // Docker and docker compose
        programs.insert(
            "docker",
            ProgramInfo {
                max_subcommand_depth: 2,
                known_subcommands: [
                    "build",
                    "compose",
                    "container",
                    "context",
                    "image",
                    "network",
                    "node",
                    "plugin",
                    "run",
                    "secret",
                    "service",
                    "stack",
                    "swarm",
                    "system",
                    "trust",
                    "volume",
                    "attach",
                    "commit",
                    "cp",
                    "create",
                    "diff",
                    "events",
                    "exec",
                    "export",
                    "history",
                    "images",
                    "import",
                    "info",
                    "inspect",
                    "kill",
                    "load",
                    "login",
                    "logout",
                    "logs",
                    "pause",
                    "port",
                    "ps",
                    "pull",
                    "push",
                    "rename",
                    "restart",
                    "rm",
                    "rmi",
                    "save",
                    "search",
                    "start",
                    "stats",
                    "stop",
                    "tag",
                    "top",
                    "unpause",
                    "update",
                    "version",
                    "wait",
                    // Compose subcommands
                    "up",
                    "down",
                    "build",
                    "config",
                    "create",
                    "events",
                    "exec",
                    "kill",
                    "logs",
                    "pause",
                    "port",
                    "ps",
                    "pull",
                    "push",
                    "restart",
                    "rm",
                    "run",
                    "scale",
                    "start",
                    "stop",
                    "top",
                    "unpause",
                ]
                .iter()
                .copied()
                .collect(),
            },
        );

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
                .copied()
                .collect(),
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
                .copied()
                .collect(),
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
                .copied()
                .collect(),
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
                .copied()
                .collect(),
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
        let mut subcommands = Vec::new();
        let mut flags = HashSet::new();
        let mut args = Vec::new();

        let program_info = self.programs.get(program);
        let max_depth = program_info.map(|p| p.max_subcommand_depth).unwrap_or(0);
        let known_subcommands = program_info.map(|p| &p.known_subcommands);

        let mut in_subcommand_region = true;
        let mut subcommand_depth = 0;

        for word in remaining {
            if word.starts_with('-') {
                // It's a flag
                in_subcommand_region = false;
                Self::parse_flags(word, &mut flags);
            } else if in_subcommand_region && subcommand_depth < max_depth {
                // Check if it's a known subcommand
                let is_subcommand = known_subcommands
                    .map(|sc| sc.contains(word.as_str()))
                    .unwrap_or(false);

                if is_subcommand {
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

        (subcommands, flags, args)
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
}
