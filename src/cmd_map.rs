use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

pub type CmdCapability = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CmdMap {
    pub cmd: String,
    #[serde(default)]
    pub subcmds: Vec<CmdMap>,
    #[serde(default)]
    pub capabilities: Vec<CmdCapability>,
    #[serde(default)]
    pub flags_add_capabilities: HashMap<String, Vec<CmdCapability>>,
    #[serde(default)]
    pub flags_remove_capabilities: HashMap<String, Vec<CmdCapability>>,
}

impl CmdMap {
    /// Recursively collect all capabilities from this command map
    fn collect_capabilities(&self, capabilities: &mut BTreeSet<String>) {
        for cap in &self.capabilities {
            capabilities.insert(cap.clone());
        }
        for caps in self.flags_add_capabilities.values() {
            for cap in caps {
                capabilities.insert(cap.clone());
            }
        }
        for subcmd in &self.subcmds {
            subcmd.collect_capabilities(capabilities);
        }
    }
}

/// All known cmd_map sources (embedded)
const EMBEDDED_CMD_MAPS: &[(&str, &str)] = &[
    ("git", include_str!("../cmds/git.toml")),
    ("docker", include_str!("../cmds/docker.toml")),
    ("grep", include_str!("../cmds/grep.toml")),
    ("cat", include_str!("../cmds/cat.toml")),
    ("less", include_str!("../cmds/less.toml")),
    ("head", include_str!("../cmds/head.toml")),
    ("tail", include_str!("../cmds/tail.toml")),
    ("kubectl", include_str!("../cmds/kubectl.toml")),
    ("terraform", include_str!("../cmds/terraform.toml")),
    ("cargo", include_str!("../cmds/cargo.toml")),
    ("az", include_str!("../cmds/az.toml")),
];

/// Get all capabilities grouped by command
pub fn get_all_capabilities() -> BTreeMap<String, BTreeSet<String>> {
    let mut result: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for (cmd, contents) in EMBEDDED_CMD_MAPS {
        if let Ok(cmd_map) = toml::from_str::<CmdMap>(contents) {
            let mut capabilities = BTreeSet::new();
            cmd_map.collect_capabilities(&mut capabilities);
            if !capabilities.is_empty() {
                result.insert(cmd.to_string(), capabilities);
            }
        }
    }

    result
}

/// Get a flat list of all unique capabilities
pub fn get_all_capabilities_flat() -> BTreeSet<String> {
    let mut result = BTreeSet::new();

    for (_, contents) in EMBEDDED_CMD_MAPS {
        if let Ok(cmd_map) = toml::from_str::<CmdMap>(contents) {
            cmd_map.collect_capabilities(&mut result);
        }
    }

    result
}
