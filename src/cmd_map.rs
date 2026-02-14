use std::collections::HashMap;

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
