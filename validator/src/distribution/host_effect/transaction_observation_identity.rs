use super::transaction_observation::HostLifecycleObservationInput;
use crate::plugin_product::lifecycle::{PackageAuthority, Version};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};

pub(crate) struct HostSurfaceLocations {
    pub(crate) installed: PathBuf,
    pub(crate) cache: PathBuf,
    pub(crate) runtime: PathBuf,
}

pub(crate) fn expected_marketplace_digest(
    input: &HostLifecycleObservationInput,
) -> Result<String, &'static str> {
    typed_digest(
        "marketplace",
        &[
            input.marketplace.clone(),
            canonical_path(&input.marketplace_source_root)?,
        ],
    )
}

pub(crate) fn expected_plugin_digest(
    input: &HostLifecycleObservationInput,
    authority: &PackageAuthority,
) -> Result<String, &'static str> {
    typed_digest(
        "plugin",
        &[
            input.plugin.clone(),
            version_text(authority.version),
            authority.package_sha256.clone(),
            authority.inventory_sha256.clone(),
            authority.candidate_id.clone(),
        ],
    )
}

pub(crate) fn observe_marketplace(
    value: &Value,
    input: &HostLifecycleObservationInput,
    required: bool,
) -> Result<Option<String>, &'static str> {
    let Some(row) = find_object(value, &|row| {
        string_field(row, &["name", "marketplace"]) == Some(input.marketplace.as_str())
    }) else {
        return if required {
            Err("Codex marketplace JSON lacks the expected marketplace row")
        } else {
            Ok(None)
        };
    };
    let source_root = string_field(row, &["source_root", "source", "path"])
        .ok_or("Codex marketplace JSON lacks a typed source root")?;
    let expected_root = canonical_path(&input.marketplace_source_root)?;
    if source_root != expected_root {
        return Err("Codex marketplace source root does not match the bound repository root");
    }
    Ok(Some(typed_digest(
        "marketplace",
        &[input.marketplace.clone(), expected_root],
    )?))
}

pub(crate) fn observe_plugin(
    value: &Value,
    input: &HostLifecycleObservationInput,
    authority: Option<&PackageAuthority>,
    target_root: &Path,
) -> Result<(Option<String>, HostSurfaceLocations), &'static str> {
    let fallback = adapter_locations(target_root, input)?;
    let Some(authority) = authority else {
        let present = find_object(value, &|row| {
            string_field(row, &["id", "plugin_id", "name"]) == Some(input.plugin.as_str())
        });
        if present.is_some() {
            return Err("Codex plugin JSON retains an unexpected candidate row");
        }
        return Ok((None, fallback));
    };
    let row = find_object(value, &|row| {
        string_field(row, &["id", "plugin_id", "name"]) == Some(input.plugin.as_str())
    })
    .ok_or("Codex plugin JSON lacks the expected plugin row")?;
    let version = string_field(row, &["version"])
        .ok_or("Codex plugin JSON lacks the typed plugin version")?;
    if version != version_text(authority.version) {
        return Err("Codex plugin version does not match the lifecycle authority");
    }
    for (key, expected) in [
        ("package_sha256", authority.package_sha256.as_str()),
        ("inventory_sha256", authority.inventory_sha256.as_str()),
        ("candidate_id", authority.candidate_id.as_str()),
    ] {
        if string_field(row, &[key]) != Some(expected) {
            return Err("Codex plugin identity does not match the lifecycle authority");
        }
    }
    let locations = HostSurfaceLocations {
        installed: bound_path(row, "installed_path", target_root)?,
        cache: bound_path(row, "cache_path", target_root)?,
        runtime: bound_path(row, "runtime_path", target_root)?,
    };
    if locations.installed == input.marketplace_source_path
        || locations.cache == input.marketplace_source_path
        || locations.runtime == input.marketplace_source_path
    {
        return Err("Codex plugin observation aliases the marketplace source");
    }
    let digest = typed_digest(
        "plugin",
        &[
            input.plugin.clone(),
            version.to_owned(),
            authority.package_sha256.clone(),
            authority.inventory_sha256.clone(),
            authority.candidate_id.clone(),
        ],
    )?;
    Ok((Some(digest), locations))
}

fn adapter_locations(
    target_root: &Path,
    input: &HostLifecycleObservationInput,
) -> Result<HostSurfaceLocations, &'static str> {
    let locations = HostSurfaceLocations {
        installed: target_root.join("plugins").join(&input.plugin),
        cache: target_root
            .join("plugins/cache")
            .join(&input.marketplace)
            .join(&input.plugin),
        runtime: target_root
            .join("plugins")
            .join(&input.plugin)
            .join("runtime/runtime-probe-bin"),
    };
    if locations.installed == input.marketplace_source_path {
        return Err("Codex adapter installed path aliases the marketplace source");
    }
    Ok(locations)
}

fn bound_path(row: &Map<String, Value>, key: &str, root: &Path) -> Result<PathBuf, &'static str> {
    let raw = string_field(row, &[key]).ok_or("Codex plugin JSON lacks a surface path")?;
    let path = PathBuf::from(raw);
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
        || !path.starts_with(root)
    {
        return Err("Codex plugin surface path is outside the confined home");
    }
    Ok(path)
}

fn typed_digest(label: &str, fields: &[String]) -> Result<String, &'static str> {
    let bytes = serde_json::to_vec(&("harness-ultragoal.host-lifecycle-surface.v1", label, fields))
        .map_err(|_| "host observation identity encoding failed")?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn canonical_path(path: &Path) -> Result<String, &'static str> {
    path.canonicalize()
        .map(|value| value.display().to_string())
        .map_err(|_| "host observation authority path unavailable")
}

fn version_text(version: Version) -> String {
    format!("{}.{}.{}", version.major, version.minor, version.patch)
}

fn string_field<'a>(row: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| row.get(*key).and_then(Value::as_str))
}

fn find_object<'a>(
    value: &'a Value,
    predicate: &dyn Fn(&Map<String, Value>) -> bool,
) -> Option<&'a Map<String, Value>> {
    match value {
        Value::Object(row) if predicate(row) => Some(row),
        Value::Object(row) => row
            .values()
            .find_map(|child| find_object(child, &predicate)),
        Value::Array(rows) => rows.iter().find_map(|child| find_object(child, &predicate)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn arbitrary_marketplace_and_plugin_json_is_rejected_before_effect_replay() {
        let root =
            std::env::temp_dir().join(format!("hul-observation-identity-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let input = HostLifecycleObservationInput {
            marketplace_source_path: root.join("plugins/source"),
            marketplace_source_root: root.clone(),
            marketplace: "local-marketplace".to_owned(),
            plugin: "harness-ultragoal".to_owned(),
        };
        let canonical_root = root.canonicalize().unwrap();
        let marketplace = json!([{
            "name": "local-marketplace",
            "source_root": canonical_root.display().to_string()
        }]);
        assert!(observe_marketplace(&marketplace, &input, true).is_ok());
        let mut altered_marketplace = marketplace.clone();
        altered_marketplace[0]["source_root"] = json!(root.join("other").display().to_string());
        assert!(observe_marketplace(&altered_marketplace, &input, true).is_err());
        let authority = PackageAuthority {
            version: Version::parse("1.2.3").unwrap(),
            package_sha256: digest('a'),
            inventory_sha256: digest('b'),
            candidate_id: digest('c'),
        };
        let plugin = json!([{
            "id": "harness-ultragoal",
            "version": "1.2.3",
            "package_sha256": authority.package_sha256,
            "inventory_sha256": authority.inventory_sha256,
            "candidate_id": authority.candidate_id,
            "installed_path": root.join("plugins/installed").display().to_string(),
            "cache_path": root.join("cache/plugin").display().to_string(),
            "runtime_path": root.join("plugins/installed/runtime/probe").display().to_string()
        }]);
        assert!(observe_plugin(&plugin, &input, Some(&authority), &root).is_ok());
        let mut altered_plugin = plugin.clone();
        altered_plugin[0]["candidate_id"] = json!(digest('d'));
        assert!(observe_plugin(&altered_plugin, &input, Some(&authority), &root).is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    fn digest(seed: char) -> String {
        format!("sha256:{}", seed.to_string().repeat(64))
    }
}
