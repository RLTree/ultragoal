use super::CONTRACT_DIR;
use super::{id, registry_entry, required_entry, rows, safe_identifier};
use crate::inventory::types::{ActiveStatus, AuthorityState, InventoryEntry, InventoryError};
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn load(
    product: &Value,
    entries: &mut Vec<InventoryEntry>,
    counts: &mut BTreeMap<String, usize>,
) -> Result<BTreeMap<String, String>, InventoryError> {
    let topology = product
        .get("component_topology")
        .ok_or_else(|| InventoryError::InvalidRegistry("missing component_topology".to_owned()))?;
    let mut legacy_skills = BTreeMap::new();
    for row in rows(topology, "canonical_skills")? {
        let name = id(row, "canonical")?;
        let mut legacy = Vec::new();
        for route in row
            .get("legacy_routes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !safe_identifier(route) {
                return Err(InventoryError::InvalidRegistry(
                    "legacy skill route has a noncanonical identifier".to_owned(),
                ));
            }
            legacy.push(route.to_owned());
            legacy_skills.insert(route.to_owned(), name.clone());
        }
        entries.push(required_entry(
            format!("SKILL:{name}"),
            "skill",
            format!("skills/{name}/SKILL.md"),
            legacy,
        ));
    }
    let mut expected_agents = std::collections::BTreeSet::new();
    for row in rows(topology, "read_only_agent_roles")? {
        let name = id(row, "name")?;
        expected_agents.insert(name.clone());
        entries.push(required_entry(
            format!("AGENT:{name}"),
            "agent",
            format!(".codex/agents/{name}.toml"),
            Vec::new(),
        ));
    }
    let compiled_agents = crate::agent_roles::CANONICAL_AGENT_ROLES
        .iter()
        .map(|role| role.name.to_owned())
        .collect::<std::collections::BTreeSet<_>>();
    if compiled_agents != expected_agents
        || crate::agent_roles::CANONICAL_AGENT_ROLES
            .iter()
            .any(|role| role.manifest_path != format!(".codex/agents/{}.toml", role.name))
    {
        return Err(InventoryError::InvalidRegistry(
            "compiled agent roles disagree with the adopted product inventory".to_owned(),
        ));
    }
    let mut expected_commands = std::collections::BTreeSet::new();
    for row in rows(topology, "cli_command_groups")? {
        let name = id(row, "name")?;
        expected_commands.insert(name.clone());
        let mut command = registry_entry(
            row,
            format!("COMMAND:{name}"),
            "command-group",
            "OWN-CLI",
            "PRODUCT_SURFACE_INVENTORY.json",
        )?;
        command.active_status = ActiveStatus::Required;
        entries.push(command);
    }
    let compiled = crate::command_witness::compiled_groups()
        .map_err(|message| InventoryError::InvalidRegistry(message.to_owned()))?;
    if compiled
        .iter()
        .map(|group| group.name.clone())
        .collect::<std::collections::BTreeSet<_>>()
        != expected_commands
    {
        return Err(InventoryError::InvalidRegistry(
            "compiled command groups disagree with the adopted product inventory".to_owned(),
        ));
    }
    for group in compiled {
        entries.push(InventoryEntry {
            stable_id: format!("COMMAND:{}", group.name),
            kind: "command-group".to_owned(),
            owner_role: "OWN-CLI".to_owned(),
            relative_path: format!(
                "{CONTRACT_DIR}/PRODUCT_SURFACE_INVENTORY.json#/command-group/COMMAND:{}",
                group.name
            ),
            digest_sha256: group.digest_sha256,
            unix_mode: None,
            authority_state: AuthorityState::Canonical,
            active_status: ActiveStatus::Candidate,
            generator: Some("HCT-INVENTORY:compiled-command-catalog-witness".to_owned()),
            input_provenance: vec![
                "validator/src/cli/successor/catalog.rs".to_owned(),
                "validator/src/cli/successor/model.rs".to_owned(),
            ],
            references: vec![format!("ROUTE-COUNT:{}", group.route_count)],
        });
    }
    counts.insert(
        "canonical_skills".to_owned(),
        rows(topology, "canonical_skills")?.len(),
    );
    counts.insert(
        "read_only_agent_roles".to_owned(),
        rows(topology, "read_only_agent_roles")?.len(),
    );
    counts.insert(
        "cli_command_groups".to_owned(),
        rows(topology, "cli_command_groups")?.len(),
    );
    Ok(legacy_skills)
}
