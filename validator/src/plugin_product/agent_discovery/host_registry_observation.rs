use super::error::{AgentDiscoveryError, AgentDiscoveryErrorId};
use super::filesystem::{digest, valid_sha256};
use serde::de::{Error as _, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};
use std::ffi::OsStr;
use std::fmt;
use std::path::{Component, Path, PathBuf};

const PLUGIN_ID: &str = "harness-ultragoal@local-harness-plugins";
const PLUGIN_NAME: &str = "harness-ultragoal";
const MARKETPLACE_NAME: &str = "local-harness-plugins";
const MAX_REGISTRY_BYTES: usize = 8 * 1024 * 1024;
const MAX_REGISTRY_ROWS: usize = 128;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HostPluginRegistryObservation {
    selected_codex_executable: PathBuf,
    selected_codex_identity_sha256: String,
    installed_root: PathBuf,
    marketplace_root: PathBuf,
    plugin_version: String,
    sha256: String,
}

impl HostPluginRegistryObservation {
    pub(crate) fn installed_root(&self) -> &Path {
        &self.installed_root
    }

    pub(crate) fn marketplace_root(&self) -> &Path {
        &self.marketplace_root
    }

    pub(crate) fn selected_codex_identity_sha256(&self) -> &str {
        &self.selected_codex_identity_sha256
    }

    pub(crate) fn sha256(&self) -> &str {
        &self.sha256
    }

    pub(crate) fn plugin_version(&self) -> &str {
        &self.plugin_version
    }

    #[cfg(test)]
    pub(crate) fn fixture(
        installed_root: &Path,
        marketplace_root: &Path,
        plugin_version: &str,
    ) -> Self {
        let sha256 = digest(
            &serde_json::to_vec(&(
                "HostPluginRegistryObservationFixture-v1",
                installed_root,
                installed_root,
                marketplace_root,
                plugin_version,
            ))
            .expect("fixture observation serializes"),
        );
        Self {
            selected_codex_executable: installed_root.to_path_buf(),
            selected_codex_identity_sha256: digest(b"fixture-selected-codex"),
            installed_root: installed_root.to_path_buf(),
            marketplace_root: marketplace_root.to_path_buf(),
            plugin_version: plugin_version.to_owned(),
            sha256,
        }
    }
}

pub(crate) fn parse_host_plugin_registry_observation(
    plugin_json: &[u8],
    marketplace_json: &[u8],
    expected_plugin_version: &str,
    selected_codex_executable: &Path,
    selected_codex_identity_sha256: &str,
) -> Result<HostPluginRegistryObservation, AgentDiscoveryError> {
    parse_host_plugin_registry_observation_inner(
        plugin_json,
        marketplace_json,
        Some(expected_plugin_version),
        selected_codex_executable,
        selected_codex_identity_sha256,
    )
}

pub(crate) fn parse_unpinned_host_plugin_registry_observation(
    plugin_json: &[u8],
    marketplace_json: &[u8],
    selected_codex_executable: &Path,
    selected_codex_identity_sha256: &str,
) -> Result<HostPluginRegistryObservation, AgentDiscoveryError> {
    parse_host_plugin_registry_observation_inner(
        plugin_json,
        marketplace_json,
        None,
        selected_codex_executable,
        selected_codex_identity_sha256,
    )
}

fn parse_host_plugin_registry_observation_inner(
    plugin_json: &[u8],
    marketplace_json: &[u8],
    expected_plugin_version: Option<&str>,
    selected_codex_executable: &Path,
    selected_codex_identity_sha256: &str,
) -> Result<HostPluginRegistryObservation, AgentDiscoveryError> {
    if plugin_json.len() > MAX_REGISTRY_BYTES
        || marketplace_json.len() > MAX_REGISTRY_BYTES
        || !valid_sha256(selected_codex_identity_sha256)
    {
        return Err(invalid());
    }
    let selected_codex_executable = canonical_observed_file(selected_codex_executable)?;
    let plugins = parse_unique_json(plugin_json)?;
    let marketplaces = parse_unique_json(marketplace_json)?;
    let plugin_rows = rows(&plugins, "installed")?;
    let marketplace_rows = rows(&marketplaces, "marketplaces")?;
    let (plugin_index, plugin) = exact_plugin_row(plugin_rows)?;
    let (marketplace_index, marketplace) = exact_marketplace_row(marketplace_rows)?;

    let plugin_version = string(plugin, "version").ok_or_else(identity)?;
    if crate::plugin_manifest::Version::parse_codex_plugin(plugin_version).is_none()
        || expected_plugin_version.is_some_and(|expected| expected != plugin_version)
        || string(plugin, "pluginId") != Some(PLUGIN_ID)
        || string(plugin, "name") != Some(PLUGIN_NAME)
        || string(plugin, "marketplaceName") != Some(MARKETPLACE_NAME)
        || boolean(plugin, "installed") != Some(true)
        || boolean(plugin, "enabled") != Some(true)
    {
        return Err(identity());
    }
    let source = plugin
        .get("source")
        .and_then(Value::as_object)
        .ok_or_else(identity)?;
    if string(source, "source") != Some("local") {
        return Err(identity());
    }
    let installed_root =
        canonical_observed_directory(string(source, "path").ok_or_else(identity)?)?;
    let marketplace_root =
        canonical_observed_directory(string(marketplace, "root").ok_or_else(identity)?)?;
    if string(marketplace, "name") != Some(MARKETPLACE_NAME)
        || installed_root == marketplace_root
        || !installed_root.starts_with(&marketplace_root)
        || installed_root.file_name().and_then(|name| name.to_str()) != Some(PLUGIN_NAME)
    {
        return Err(identity());
    }
    reject_plugin_aliases(plugin_rows, plugin_index, &installed_root)?;
    reject_marketplace_aliases(marketplace_rows, marketplace_index, &marketplace_root)?;

    let sha256 = digest(
        &serde_json::to_vec(&(
            "HostPluginRegistryObservation-v1",
            &selected_codex_executable,
            selected_codex_identity_sha256,
            &plugins,
            &marketplaces,
            &installed_root,
            &marketplace_root,
        ))
        .map_err(|_| invalid())?,
    );
    Ok(HostPluginRegistryObservation {
        selected_codex_executable,
        selected_codex_identity_sha256: selected_codex_identity_sha256.to_owned(),
        installed_root,
        marketplace_root,
        plugin_version: plugin_version.to_owned(),
        sha256,
    })
}

fn exact_plugin_row(rows: &[Value]) -> Result<(usize, &Map<String, Value>), AgentDiscoveryError> {
    exact_row(rows, |row| {
        string(row, "pluginId") == Some(PLUGIN_ID) || string(row, "name") == Some(PLUGIN_NAME)
    })
}

fn exact_marketplace_row(
    rows: &[Value],
) -> Result<(usize, &Map<String, Value>), AgentDiscoveryError> {
    exact_row(rows, |row| string(row, "name") == Some(MARKETPLACE_NAME))
}

fn rows<'a>(value: &'a Value, collection: &str) -> Result<&'a [Value], AgentDiscoveryError> {
    let rows = value
        .as_object()
        .and_then(|root| root.get(collection))
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;
    if rows.len() > MAX_REGISTRY_ROWS {
        return Err(AgentDiscoveryError::new(
            AgentDiscoveryErrorId::InputTooLarge,
        ));
    }
    Ok(rows)
}

fn exact_row(
    rows: &[Value],
    matches: impl Fn(&Map<String, Value>) -> bool,
) -> Result<(usize, &Map<String, Value>), AgentDiscoveryError> {
    let mut selected = None;
    for (index, value) in rows.iter().enumerate() {
        let row = value.as_object().ok_or_else(invalid)?;
        if matches(row) && selected.replace((index, row)).is_some() {
            return Err(conflict());
        }
    }
    selected.ok_or_else(|| AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationUnavailable))
}

fn reject_plugin_aliases(
    rows: &[Value],
    selected_index: usize,
    installed_root: &Path,
) -> Result<(), AgentDiscoveryError> {
    for (index, value) in rows.iter().enumerate() {
        if index == selected_index {
            continue;
        }
        let row = value.as_object().ok_or_else(invalid)?;
        if [("pluginId", PLUGIN_ID), ("name", PLUGIN_NAME)]
            .into_iter()
            .any(|(key, expected)| string(row, key).is_some_and(|value| alias(value, expected)))
        {
            return Err(conflict());
        }
        if row
            .get("source")
            .and_then(Value::as_object)
            .and_then(|source| string(source, "path"))
            .and_then(canonical_directory_if_present)
            .is_some_and(|path| path == installed_root)
        {
            return Err(conflict());
        }
    }
    Ok(())
}

fn reject_marketplace_aliases(
    rows: &[Value],
    selected_index: usize,
    marketplace_root: &Path,
) -> Result<(), AgentDiscoveryError> {
    for (index, value) in rows.iter().enumerate() {
        if index == selected_index {
            continue;
        }
        let row = value.as_object().ok_or_else(invalid)?;
        if string(row, "name").is_some_and(|value| alias(value, MARKETPLACE_NAME))
            || string(row, "root")
                .and_then(canonical_directory_if_present)
                .is_some_and(|path| path == marketplace_root)
        {
            return Err(conflict());
        }
    }
    Ok(())
}

fn alias(value: &str, expected: &str) -> bool {
    value
        .bytes()
        .filter(u8::is_ascii_alphanumeric)
        .map(|byte| byte.to_ascii_lowercase())
        .eq(expected
            .bytes()
            .filter(u8::is_ascii_alphanumeric)
            .map(|byte| byte.to_ascii_lowercase()))
}

fn canonical_directory_if_present(value: &str) -> Option<PathBuf> {
    let path = Path::new(value);
    path.is_absolute()
        .then(|| path.canonicalize().ok())
        .flatten()
}

fn canonical_observed_directory(value: &str) -> Result<PathBuf, AgentDiscoveryError> {
    if value.is_empty() || value.len() > 4096 || value.chars().any(char::is_control) {
        return Err(invalid());
    }
    let path = Path::new(value);
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err(identity());
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationUnavailable))?;
    if canonical.as_os_str() != OsStr::new(value)
        || !std::fs::symlink_metadata(&canonical)
            .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
    {
        return Err(identity());
    }
    Ok(canonical)
}

fn canonical_observed_file(path: &Path) -> Result<PathBuf, AgentDiscoveryError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err(identity());
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationUnavailable))?;
    let metadata = std::fs::symlink_metadata(&canonical)
        .map_err(|_| AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationUnavailable))?;
    if canonical.as_os_str() != path.as_os_str()
        || metadata.file_type().is_symlink()
        || !metadata.is_file()
    {
        return Err(identity());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if metadata.nlink() != 1 || metadata.permissions().mode() & 0o111 == 0 {
            return Err(identity());
        }
    }
    Ok(canonical)
}

fn parse_unique_json(bytes: &[u8]) -> Result<Value, AgentDiscoveryError> {
    serde_json::from_slice::<UniqueValue>(bytes)
        .map(|value| value.0)
        .map_err(|_| invalid())
}

struct UniqueValue(Value);

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(UniqueValueVisitor)
    }
}

struct UniqueValueVisitor;

impl<'de> Visitor<'de> for UniqueValueVisitor {
    type Value = UniqueValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON with unique object keys")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Number(value.into())))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Number(value.into())))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .map(UniqueValue)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::String(value.to_owned())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::String(value)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Null))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Null))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<UniqueValue>()? {
            values.push(value.0);
        }
        Ok(UniqueValue(Value::Array(values)))
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        while let Some((key, value)) = object.next_entry::<String, UniqueValue>()? {
            if values.insert(key.clone(), value.0).is_some() {
                return Err(A::Error::custom(format!("duplicate object key {key:?}")));
            }
        }
        Ok(UniqueValue(Value::Object(values)))
    }
}

fn string<'a>(row: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    row.get(key).and_then(Value::as_str)
}

fn boolean(row: &Map<String, Value>, key: &str) -> Option<bool> {
    row.get(key).and_then(Value::as_bool)
}

fn invalid() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::InvalidBinding)
}

fn identity() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::IdentityMismatch)
}

fn conflict() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationConflict)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(1);
    const CURRENT_VERSION: &str = "0.0.41+codex.20260824093100";

    #[test]
    fn exact_enabled_local_rows_select_one_canonical_installed_root() {
        let fixture = Fixture::new();
        let observation = parse_host_plugin_registry_observation(
            &fixture.plugin_json(),
            &fixture.marketplace_json(),
            CURRENT_VERSION,
            &fixture.codex,
            &digest(b"selected-codex"),
        )
        .unwrap();
        assert_eq!(observation.installed_root(), fixture.plugin.as_path());
        assert!(valid_sha256(observation.sha256()));
    }

    #[test]
    fn unpinned_read_only_observation_seals_the_registry_version_without_selecting_it() {
        let fixture = Fixture::new();
        let observation = parse_unpinned_host_plugin_registry_observation(
            &fixture.plugin_json(),
            &fixture.marketplace_json(),
            &fixture.codex,
            &digest(b"selected-codex"),
        )
        .unwrap();
        assert_eq!(observation.plugin_version(), CURRENT_VERSION);
        let malformed = serde_json::to_vec(&json!({
            "installed": [merge(&fixture.plugin_row(), "version", json!("latest"))]
        }))
        .unwrap();
        assert!(
            parse_unpinned_host_plugin_registry_observation(
                &malformed,
                &fixture.marketplace_json(),
                &fixture.codex,
                &digest(b"selected-codex"),
            )
            .is_err()
        );
    }

    #[test]
    fn registry_observation_accepts_and_seals_the_live_plugin_creator_version() {
        let fixture = Fixture::new();
        let version = "0.0.39+codex.20260820190706";
        let plugin_json = serde_json::to_vec(&json!({
            "installed": [merge(&fixture.plugin_row(), "version", json!(version))]
        }))
        .unwrap();
        let observation = parse_unpinned_host_plugin_registry_observation(
            &plugin_json,
            &fixture.marketplace_json(),
            &fixture.codex,
            &digest(b"selected-codex"),
        )
        .unwrap();
        assert_eq!(observation.plugin_version(), version);
        assert!(
            parse_host_plugin_registry_observation(
                &plugin_json,
                &fixture.marketplace_json(),
                version,
                &fixture.codex,
                &digest(b"selected-codex"),
            )
            .is_ok()
        );

        let substituted_version = "0.0.39+codex.substituted-cachebuster";
        let substituted_json = serde_json::to_vec(&json!({
            "installed": [merge(
                &fixture.plugin_row(),
                "version",
                json!(substituted_version),
            )]
        }))
        .unwrap();
        let substituted = parse_unpinned_host_plugin_registry_observation(
            &substituted_json,
            &fixture.marketplace_json(),
            &fixture.codex,
            &digest(b"selected-codex"),
        )
        .unwrap();
        assert_ne!(observation.sha256(), substituted.sha256());
        assert!(
            parse_host_plugin_registry_observation(
                &plugin_json,
                &fixture.marketplace_json(),
                substituted_version,
                &fixture.codex,
                &digest(b"selected-codex"),
            )
            .is_err()
        );

        for invalid in [
            "0.0.39+other.token",
            "0.0.39+codex.",
            "0.0.39+codex.-token",
            "0.0.39+codex.token-",
            "0.0.39+codex.two--hyphens",
            "0.0.39+codex.UPPER",
            "0.0.39+codex.two.parts",
            "0.0.39+codex.token+extra",
        ] {
            let malformed = serde_json::to_vec(&json!({
                "installed": [merge(&fixture.plugin_row(), "version", json!(invalid))]
            }))
            .unwrap();
            assert!(
                parse_unpinned_host_plugin_registry_observation(
                    &malformed,
                    &fixture.marketplace_json(),
                    &fixture.codex,
                    &digest(b"selected-codex"),
                )
                .is_err(),
                "{invalid}"
            );
        }
    }

    #[test]
    fn missing_duplicate_disabled_remote_wrong_version_and_alias_rows_fail_closed() {
        let fixture = Fixture::new();
        let exact = fixture.plugin_row();
        for installed in [
            json!([]),
            json!([exact.clone(), exact.clone()]),
            json!([merge(&exact, "enabled", json!(false))]),
            json!([merge(&exact, "version", json!("0.0.39"))]),
            json!([merge(
                &exact,
                "source",
                json!({"source":"git","path":fixture.plugin})
            )]),
            json!([
                exact.clone(),
                {"pluginId":"alias@other","name":PLUGIN_NAME}
            ]),
        ] {
            let bytes = serde_json::to_vec(&json!({"installed": installed})).unwrap();
            assert!(
                parse_host_plugin_registry_observation(
                    &bytes,
                    &fixture.marketplace_json(),
                    CURRENT_VERSION,
                    &fixture.codex,
                    &digest(b"selected-codex"),
                )
                .is_err()
            );
        }
    }

    #[test]
    fn duplicate_wrong_and_noncanonical_marketplace_rows_fail_closed() {
        let fixture = Fixture::new();
        let canonical = json!({"name":MARKETPLACE_NAME,"root":fixture.root});
        for marketplaces in [
            json!([]),
            json!([canonical.clone(), canonical.clone()]),
            json!([{"name":MARKETPLACE_NAME,"root":fixture.root.join(".")}]),
            json!([{"name":MARKETPLACE_NAME,"root":fixture.plugin}]),
        ] {
            let bytes = serde_json::to_vec(&json!({"marketplaces": marketplaces})).unwrap();
            assert!(
                parse_host_plugin_registry_observation(
                    &fixture.plugin_json(),
                    &bytes,
                    CURRENT_VERSION,
                    &fixture.codex,
                    &digest(b"selected-codex"),
                )
                .is_err()
            );
        }
    }

    #[test]
    fn selected_executable_path_and_digest_are_sealed_and_aliases_are_rejected() {
        let fixture = Fixture::new();
        let second = fixture.root.join("codex-second");
        fs::copy(&fixture.codex, &second).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::{PermissionsExt, symlink};
            fs::set_permissions(&second, fs::Permissions::from_mode(0o700)).unwrap();
            let alias = fixture.root.join("codex-alias");
            symlink(&fixture.codex, &alias).unwrap();
            assert!(
                parse_host_plugin_registry_observation(
                    &fixture.plugin_json(),
                    &fixture.marketplace_json(),
                    CURRENT_VERSION,
                    &alias,
                    &digest(b"selected-codex"),
                )
                .is_err()
            );
        }
        let first = fixture.observation(&fixture.codex, &digest(b"selected-codex"));
        let changed_path = fixture.observation(&second, &digest(b"selected-codex"));
        let changed_digest = fixture.observation(&fixture.codex, &digest(b"other-codex"));
        assert_ne!(first, changed_path);
        assert_ne!(first, changed_digest);
    }

    #[test]
    fn alternate_plugin_and_marketplace_identities_cannot_alias_selected_roots() {
        let fixture = Fixture::new();
        let exact = fixture.plugin_row();
        for alias_row in [
            json!({
                "pluginId":"other@marketplace",
                "name":"other",
                "source":{"source":"local","path":fixture.plugin}
            }),
            json!({"pluginId":"HARNESS_ULTRAGOAL@LOCAL_HARNESS_PLUGINS","name":"other"}),
            json!({"pluginId":"other@marketplace","name":"Harness_Ultragoal"}),
        ] {
            let plugins = serde_json::to_vec(&json!({
                "installed":[exact.clone(), alias_row]
            }))
            .unwrap();
            assert!(
                fixture
                    .parse(&plugins, &fixture.marketplace_json())
                    .is_err()
            );
        }
        for alias_row in [
            json!({"name":"other-marketplace","root":fixture.root}),
            json!({"name":"LOCAL_HARNESS_PLUGINS","root":fixture.root.join("other")}),
        ] {
            let marketplaces = serde_json::to_vec(&json!({
                "marketplaces":[
                    {"name":MARKETPLACE_NAME,"root":fixture.root},
                    alias_row
                ]
            }))
            .unwrap();
            assert!(
                fixture
                    .parse(&fixture.plugin_json(), &marketplaces)
                    .is_err()
            );
        }
    }

    #[test]
    fn duplicate_json_keys_at_every_authority_relevant_depth_fail_closed() {
        let fixture = Fixture::new();
        let plugin_path = serde_json::to_string(&fixture.plugin).unwrap();
        let marketplace_path = serde_json::to_string(&fixture.root).unwrap();
        let plugin_cases = [
            format!(
                "{{\"installed\":[],\"installed\":[{}]}}",
                serde_json::to_string(&fixture.plugin_row()).unwrap()
            ),
            format!(
                "{{\"installed\":[{{\"pluginId\":{PLUGIN_ID:?},\"name\":{PLUGIN_NAME:?},\"marketplaceName\":{MARKETPLACE_NAME:?},\"version\":\"{CURRENT_VERSION}\",\"installed\":true,\"enabled\":false,\"enabled\":true,\"source\":{{\"source\":\"local\",\"path\":{plugin_path}}}}}]}}"
            ),
            format!(
                "{{\"installed\":[{{\"pluginId\":{PLUGIN_ID:?},\"name\":{PLUGIN_NAME:?},\"marketplaceName\":{MARKETPLACE_NAME:?},\"version\":\"{CURRENT_VERSION}\",\"installed\":true,\"enabled\":true,\"source\":{{\"source\":\"local\",\"path\":{plugin_path},\"path\":{plugin_path}}}}}]}}"
            ),
        ];
        for plugin_json in plugin_cases {
            assert!(
                fixture
                    .parse(plugin_json.as_bytes(), &fixture.marketplace_json())
                    .is_err()
            );
        }
        let marketplace_json = format!(
            "{{\"marketplaces\":[{{\"name\":{MARKETPLACE_NAME:?},\"root\":{marketplace_path},\"root\":{marketplace_path}}}]}}"
        );
        assert!(
            fixture
                .parse(&fixture.plugin_json(), marketplace_json.as_bytes())
                .is_err()
        );
    }

    struct Fixture {
        root: PathBuf,
        plugin: PathBuf,
        codex: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "hul-plugin-registry-observation-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let plugin = root.join("plugins/harness-ultragoal");
            fs::create_dir_all(&plugin).unwrap();
            let codex = root.join("codex");
            fs::write(&codex, b"#!/bin/sh\nexit 0\n").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&codex, fs::Permissions::from_mode(0o700)).unwrap();
            }
            Self {
                root: root.canonicalize().unwrap(),
                plugin: plugin.canonicalize().unwrap(),
                codex: codex.canonicalize().unwrap(),
            }
        }

        fn plugin_row(&self) -> Value {
            json!({
                "pluginId": PLUGIN_ID,
                "name": PLUGIN_NAME,
                "marketplaceName": MARKETPLACE_NAME,
                "version": CURRENT_VERSION,
                "installed": true,
                "enabled": true,
                "source": {"source":"local","path":self.plugin},
                "installPolicy":"AVAILABLE",
                "authPolicy":"ON_INSTALL"
            })
        }

        fn plugin_json(&self) -> Vec<u8> {
            serde_json::to_vec(&json!({"installed":[self.plugin_row()]})).unwrap()
        }

        fn marketplace_json(&self) -> Vec<u8> {
            serde_json::to_vec(&json!({
                "marketplaces":[{"name":MARKETPLACE_NAME,"root":self.root}]
            }))
            .unwrap()
        }

        fn observation(&self, codex: &Path, codex_sha256: &str) -> HostPluginRegistryObservation {
            parse_host_plugin_registry_observation(
                &self.plugin_json(),
                &self.marketplace_json(),
                CURRENT_VERSION,
                codex,
                codex_sha256,
            )
            .unwrap()
        }

        fn parse(
            &self,
            plugin_json: &[u8],
            marketplace_json: &[u8],
        ) -> Result<HostPluginRegistryObservation, AgentDiscoveryError> {
            parse_host_plugin_registry_observation(
                plugin_json,
                marketplace_json,
                CURRENT_VERSION,
                &self.codex,
                &digest(b"selected-codex"),
            )
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn merge(value: &Value, key: &str, replacement: Value) -> Value {
        let mut value = value.clone();
        value[key] = replacement;
        value
    }
}
