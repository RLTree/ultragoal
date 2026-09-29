use super::model::{
    SupportedAgentAuthorityFinding, SupportedAgentAuthorityFindingKind,
    SupportedAgentAuthorityObservation,
};
use super::state::{ReportState, lock_report};
use crate::plugin_product::agent_discovery::filesystem::{
    parse_descriptor, parse_global_descriptor,
};
use crate::plugin_product::agent_discovery::model::{
    AgentAuthorityLayer, PLUGIN_NAME, RawAgentRow,
};
use crate::plugin_product::agent_discovery::source::SourceAgentCatalog;
use crate::plugin_product::agent_discovery::supported::roots::LayerFiles;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

pub(in super::super) fn record_layer(
    report: &Arc<Mutex<ReportState>>,
    source: &SourceAgentCatalog,
    layer: AgentAuthorityLayer,
    files: &LayerFiles,
    agents: &[RawAgentRow],
    manifest_matches: bool,
) {
    let canonical = source
        .canonical_agents()
        .into_iter()
        .map(|row| row.name().to_owned())
        .collect::<BTreeSet<_>>();
    let canonical_normalized = canonical
        .iter()
        .map(|name| normalized_name(name))
        .collect::<BTreeSet<_>>();
    let observed = agents
        .iter()
        .map(|row| row.name.clone())
        .collect::<BTreeSet<_>>();
    let mut normalized_seen = BTreeSet::new();
    let mut state = lock_report(report);
    if files.manifest().is_some() && !manifest_matches {
        state.insert_finding(finding(
            layer,
            SupportedAgentAuthorityFindingKind::PluginIdentityMismatch,
            PLUGIN_NAME,
        ));
    }
    for row in agents {
        let sandbox_mode = if layer == AgentAuthorityLayer::Global {
            parse_global_descriptor(row.descriptor_toml.as_bytes())
                .ok()
                .and_then(|descriptor| descriptor.sandbox_mode)
        } else {
            parse_descriptor(row.descriptor_toml.as_bytes())
                .ok()
                .map(|descriptor| descriptor.sandbox_mode)
        };
        state.insert_observation(SupportedAgentAuthorityObservation {
            layer,
            authority_root_sha256: files.authority_root_sha256().to_owned(),
            authority_name: row.name.clone(),
            manifest_path: row.manifest_path.clone(),
            descriptor_sha256: row.descriptor_sha256.clone(),
            sandbox_mode: sandbox_mode.clone(),
        });
        let normalized = normalized_name(&row.name);
        if !normalized_seen.insert(normalized.clone()) {
            state.insert_finding(finding(
                layer,
                SupportedAgentAuthorityFindingKind::NormalizedCollision,
                &row.name,
            ));
        }
        if layer == AgentAuthorityLayer::Global
            && (canonical_normalized.contains(&normalized) || closed_wrapper_alias(&normalized))
        {
            state.insert_finding(finding(
                layer,
                SupportedAgentAuthorityFindingKind::NormalizedCollision,
                &row.name,
            ));
        } else if layer != AgentAuthorityLayer::Global && !canonical.contains(&row.name) {
            state.insert_finding(finding(
                layer,
                SupportedAgentAuthorityFindingKind::ExtraAuthority,
                &row.name,
            ));
        }
        match (layer, sandbox_mode.as_deref()) {
            (AgentAuthorityLayer::Global, _) => {}
            (_, None) => {
                state.insert_finding(finding(
                    layer,
                    SupportedAgentAuthorityFindingKind::SandboxPolicyMissing,
                    &row.name,
                ));
                state.insert_finding(finding(
                    layer,
                    SupportedAgentAuthorityFindingKind::InvalidDescriptor,
                    &row.name,
                ));
            }
            (_, Some("read-only")) => {}
            (_, Some(_)) => state.insert_finding(finding(
                layer,
                SupportedAgentAuthorityFindingKind::WriteCapableSandbox,
                &row.name,
            )),
        }
    }
    if layer != AgentAuthorityLayer::Global {
        for name in canonical.difference(&observed) {
            state.insert_finding(finding(
                layer,
                SupportedAgentAuthorityFindingKind::MissingCanonical,
                name,
            ));
        }
    }
}

fn finding(
    layer: AgentAuthorityLayer,
    kind: SupportedAgentAuthorityFindingKind,
    authority_name: &str,
) -> SupportedAgentAuthorityFinding {
    SupportedAgentAuthorityFinding {
        layer,
        kind,
        authority_name: authority_name.to_owned(),
    }
}

fn normalized_name(value: &str) -> String {
    value
        .bytes()
        .filter(|byte| byte.is_ascii_alphanumeric())
        .map(char::from)
        .collect()
}

fn closed_wrapper_alias(normalized: &str) -> bool {
    matches!(normalized, "harness" | "ultragoal" | "harnessultragoal")
}
