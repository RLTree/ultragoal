use super::transaction_observation::HostLifecycleObservationInput;
use super::transaction_observation_rows::{installed_plugin_row, marketplace_row, string_field};
use crate::plugin_product::lifecycle::{PackageAuthority, Version};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub(crate) struct HostSurfaceLocations {
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
    let Some(row) = marketplace_row(value, &input.marketplace)? else {
        return if required {
            Err("Codex marketplace JSON lacks the expected marketplace row")
        } else {
            Ok(None)
        };
    };
    let source_root =
        marketplace_root(row).ok_or("Codex marketplace JSON lacks a typed source root")?;
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
    let Some(authority) = authority else {
        let present = installed_plugin_row(value, &input.plugin)?;
        if present.is_some() {
            return Err("Codex plugin JSON retains an unexpected candidate row");
        }
        return Ok((None, adapter_locations(target_root, input)?));
    };
    let row = installed_plugin_row(value, &input.plugin)?
        .ok_or("Codex plugin JSON lacks the expected plugin row")?;
    let version = string_field(row, &["version"])
        .ok_or("Codex plugin JSON lacks the typed plugin version")?;
    if version != version_text(authority.version) {
        return Err("Codex plugin version does not match the lifecycle authority");
    }
    if string_field(row, &["marketplaceName", "marketplace"]) != Some(input.marketplace.as_str()) {
        return Err("Codex plugin marketplace does not match the lifecycle authority");
    }
    let source = plugin_source_path(row).ok_or("Codex plugin JSON lacks a typed source path")?;
    if canonical_path(Path::new(source))? != canonical_path(&input.marketplace_source_path)? {
        return Err("Codex plugin source does not match the materialized package authority");
    }
    let locations = installed_cache_locations(target_root, input, version);
    if locations.cache == input.marketplace_source_path
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
        cache: target_root
            .join("plugins/cache")
            .join(&input.marketplace)
            .join(&input.plugin),
        runtime: target_root
            .join("plugins")
            .join(&input.plugin)
            .join("runtime/ultragoal"),
    };
    if locations.cache == input.marketplace_source_path {
        return Err("Codex adapter installed path aliases the marketplace source");
    }
    Ok(locations)
}

fn installed_cache_locations(
    target_root: &Path,
    input: &HostLifecycleObservationInput,
    version: &str,
) -> HostSurfaceLocations {
    let cache = target_root
        .join("plugins/cache")
        .join(&input.marketplace)
        .join(&input.plugin)
        .join(version);
    HostSurfaceLocations {
        cache: cache.clone(),
        runtime: cache.join("runtime/ultragoal"),
    }
}

fn marketplace_root(row: &Map<String, Value>) -> Option<&str> {
    string_field(row, &["root", "source_root"])
        .or_else(|| nested_string_field(row, "marketplaceSource", &["source"]))
}

fn plugin_source_path(row: &Map<String, Value>) -> Option<&str> {
    nested_string_field(row, "source", &["path"])
}

fn nested_string_field<'a>(
    row: &'a Map<String, Value>,
    container: &str,
    keys: &[&str],
) -> Option<&'a str> {
    row.get(container)
        .and_then(Value::as_object)
        .and_then(|nested| string_field(nested, keys))
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
        std::fs::create_dir_all(&input.marketplace_source_path).unwrap();
        let canonical_root = root.canonicalize().unwrap();
        let marketplace = json!({"marketplaces": [{
            "name": "local-marketplace",
            "source_root": canonical_root.display().to_string()
        }]});
        assert!(observe_marketplace(&marketplace, &input, true).is_ok());
        let mut altered_marketplace = marketplace.clone();
        altered_marketplace["marketplaces"][0]["source_root"] =
            json!(root.join("other").display().to_string());
        assert!(observe_marketplace(&altered_marketplace, &input, true).is_err());
        let authority = PackageAuthority {
            version: Version::parse("1.2.3").unwrap(),
            package_sha256: digest('a'),
            inventory_sha256: digest('b'),
            candidate_id: digest('c'),
        };
        let plugin = json!({"installed": [{
            "id": "harness-ultragoal",
            "version": "1.2.3",
            "marketplaceName": "local-marketplace",
            "source": {
                "source": "local",
                "path": input.marketplace_source_path.display().to_string()
            }
        }]});
        assert!(observe_plugin(&plugin, &input, Some(&authority), &root).is_ok());
        let mut altered_plugin = plugin.clone();
        altered_plugin["installed"][0]["source"]["path"] =
            json!(root.join("other").display().to_string());
        assert!(observe_plugin(&altered_plugin, &input, Some(&authority), &root).is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    fn digest(seed: char) -> String {
        format!("sha256:{}", seed.to_string().repeat(64))
    }
}
