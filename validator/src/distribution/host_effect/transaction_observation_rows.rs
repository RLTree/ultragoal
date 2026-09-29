use serde_json::{Map, Value};

pub(super) fn marketplace_row<'a>(
    value: &'a Value,
    marketplace: &str,
    expected_root: &str,
) -> Result<Option<&'a Map<String, Value>>, &'static str> {
    exact_row(value, "marketplaces", |row| {
        string_field(row, &["name", "marketplace"]) == Some(marketplace)
            && marketplace_root(row) == Some(expected_root)
    })
}

pub(super) fn installed_plugin_row<'a>(
    value: &'a Value,
    plugin: &str,
) -> Result<Option<&'a Map<String, Value>>, &'static str> {
    exact_row(value, "installed", |row| {
        ["id", "pluginId", "plugin_id", "name"]
            .iter()
            .any(|key| row.get(*key).and_then(Value::as_str) == Some(plugin))
    })
}

pub(super) fn string_field<'a>(row: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| row.get(*key).and_then(Value::as_str))
}

fn marketplace_root(row: &Map<String, Value>) -> Option<&str> {
    string_field(row, &["root", "source_root"]).or_else(|| {
        row.get("marketplaceSource")
            .and_then(Value::as_object)
            .and_then(|source| string_field(source, &["source"]))
    })
}

fn exact_row<'a>(
    value: &'a Value,
    container: &str,
    matches: impl Fn(&Map<String, Value>) -> bool,
) -> Result<Option<&'a Map<String, Value>>, &'static str> {
    let rows = value
        .as_object()
        .and_then(|root| root.get(container))
        .and_then(Value::as_array)
        .ok_or("Codex JSON lacks the required top-level row collection")?;
    let mut selected = None;
    for value in rows {
        let row = value
            .as_object()
            .ok_or("Codex JSON row collection contains a non-object entry")?;
        if matches(row) {
            if selected.replace(row).is_some() {
                return Err("Codex JSON contains duplicate matching rows");
            }
        }
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn row_selection_rejects_nested_duplicates_and_invalid_envelopes() {
        assert!(
            installed_plugin_row(
                &json!({"installed": [{"pluginId": "expected@market", "name": "expected"}]}),
                "expected"
            )
            .unwrap()
            .is_some()
        );
        assert_eq!(
            marketplace_row(&json!({"marketplaces": []}), "expected", "/expected").unwrap(),
            None
        );
        assert!(
            marketplace_row(
                &json!({"metadata": {"marketplaces": [{"name": "expected"}]}}),
                "expected",
                "/expected"
            )
            .is_err()
        );
        assert!(
            marketplace_row(
                &json!({"marketplaces": ["expected"]}),
                "expected",
                "/expected"
            )
            .is_err()
        );
        assert!(
            installed_plugin_row(
                &json!({"installed": [{"name": "expected"}, {"name": "expected"}]}),
                "expected"
            )
            .is_err()
        );
    }

    #[test]
    fn marketplace_selection_binds_name_and_root_without_accepting_exact_duplicates() {
        let value = json!({
            "marketplaces": [
                {"name": "expected", "root": "/other"},
                {
                    "name": "expected",
                    "marketplaceSource": {"source": "/expected"}
                }
            ]
        });
        assert_eq!(
            marketplace_row(&value, "expected", "/expected")
                .unwrap()
                .and_then(|row| marketplace_root(row)),
            Some("/expected")
        );
        assert!(
            marketplace_row(
                &json!({
                    "marketplaces": [
                        {"name": "expected", "root": "/expected"},
                        {"name": "expected", "root": "/expected"}
                    ]
                }),
                "expected",
                "/expected"
            )
            .is_err()
        );
    }
}
