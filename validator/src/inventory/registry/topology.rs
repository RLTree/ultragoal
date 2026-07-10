use super::{id, registry_entry, required_entry, rows, safe_identifier};
use crate::inventory::types::{ActiveStatus, InventoryEntry, InventoryError};
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
    for row in rows(topology, "read_only_agent_roles")? {
        let name = id(row, "name")?;
        entries.push(required_entry(
            format!("AGENT:{name}"),
            "agent",
            format!(".codex/agents/{name}.toml"),
            Vec::new(),
        ));
    }
    for row in rows(topology, "cli_command_groups")? {
        let name = id(row, "name")?;
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
