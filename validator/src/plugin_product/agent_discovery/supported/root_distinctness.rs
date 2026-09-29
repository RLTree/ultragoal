use super::roots::{GlobalAuthorityRoot, PluginAuthorityRoot};
use crate::plugin_product::agent_discovery::error::{AgentDiscoveryError, AgentDiscoveryErrorId};
use std::collections::BTreeSet;

pub(super) fn require_distinct_authority_roots(
    package: &PluginAuthorityRoot,
    installed: &PluginAuthorityRoot,
    cache: &PluginAuthorityRoot,
    global: &GlobalAuthorityRoot,
    project: &PluginAuthorityRoot,
) -> Result<(), AgentDiscoveryError> {
    let paths = [
        package.root.canonical_path(),
        installed.root.canonical_path(),
        cache.root.canonical_path(),
        global.root.canonical_path(),
        project.root.canonical_path(),
    ];
    if paths.iter().collect::<BTreeSet<_>>().len() != paths.len() {
        return Err(AgentDiscoveryError::new(
            AgentDiscoveryErrorId::IdentityMismatch,
        ));
    }
    Ok(())
}
