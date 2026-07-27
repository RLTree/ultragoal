use super::envelope_codec::{Categories, categories_value, empty_categories};
use crate::inventory::types::InventoryError;
use serde_json::Value;

pub(super) fn derive(
    registry: &Value,
    lanes: &[Value],
    scopes: &[Value],
    lane: &Value,
) -> Result<Value, InventoryError> {
    let mut values = empty_categories();
    add(
        &mut values,
        "files",
        strings(registry.pointer("/root_freeze/permitted_root_paths"))?,
    )?;
    for dependency in strings(lane.get("dependencies"))? {
        let dependency = find(lanes, "id", &dependency, "lease dependency is missing")?;
        for scope_id in strings(dependency.get("scope_ids"))? {
            let scope = find(scopes, "scope_id", &scope_id, "dependency scope is missing")?;
            add(&mut values, "files", strings(scope.get("contract_roots"))?)?;
            add(&mut values, "files", strings(scope.get("owned_roots"))?)?;
            add(
                &mut values,
                "generated_outputs",
                strings(scope.get("generated_roots"))?,
            )?;
            add(
                &mut values,
                "fixtures",
                strings(scope.get("fixture_roots"))?,
            )?;
            add(&mut values, "symbols", strings(scope.get("owned_symbols"))?)?;
            add(&mut values, "effects", strings(scope.get("effects"))?)?;
        }
    }
    Ok(categories_value(values))
}
pub(super) fn validate_dependencies(
    consumed: &Value,
    lane: &Value,
    lanes: &[Value],
) -> Result<(), InventoryError> {
    let expected = strings(lane.get("dependencies"))?
        .into_iter()
        .map(|id| {
            find(lanes, "id", &id, "lease dependency is missing")?
                .get("current_identity")
                .cloned()
                .filter(|value| !value.is_null())
                .ok_or_else(|| invalid("lane dependency identity is missing"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if consumed
        .get("dependency_identities")
        .and_then(Value::as_array)
        != Some(&expected)
    {
        return Err(invalid("lease dependency identities are stale"));
    }
    Ok(())
}
fn add(
    values: &mut Categories,
    name: &'static str,
    rows: Vec<String>,
) -> Result<(), InventoryError> {
    values
        .get_mut(name)
        .ok_or_else(|| invalid("lease category is unknown"))?
        .extend(rows);
    Ok(())
}
fn strings(value: Option<&Value>) -> Result<Vec<String>, InventoryError> {
    value
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("lease category is missing"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| invalid("lease category is malformed"))
        })
        .collect()
}
fn find<'a>(
    rows: &'a [Value],
    field: &str,
    value: &str,
    message: &str,
) -> Result<&'a Value, InventoryError> {
    rows.iter()
        .find(|row| row.get(field).and_then(Value::as_str) == Some(value))
        .ok_or_else(|| invalid(message))
}
fn invalid(message: &str) -> InventoryError {
    InventoryError::InvalidRegistry(message.to_owned())
}
