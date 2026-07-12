use super::RegistryData;
use crate::context::{ContextError, ReadSession};
use crate::inventory::digest::sha256_hex;
use crate::inventory::types::{
    ActivationFailure, ActiveStatus, InventoryEntry, InventoryError, InventoryFinding,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const MAX_WITNESS_SOURCE_BYTES: u64 = 4 * 1024 * 1024;
const API_GENERATOR: &str = "HCT-INVENTORY:compiled-api-witness";
const COMMAND_GENERATOR: &str = "HCT-INVENTORY:compiled-command-catalog-witness";

#[derive(Clone, Copy, Debug)]
struct WitnessSource {
    relative: &'static str,
    embedded: &'static [u8],
    responsibility: &'static str,
}

macro_rules! source {
    ($relative:literal, $embedded:expr, $responsibility:literal) => {
        WitnessSource {
            relative: $relative,
            embedded: $embedded,
            responsibility: $responsibility,
        }
    };
}

// This is the direct source closure for API and command activation. It binds
// declaration, registry construction, exact-row gating, status finalization,
// public serialization, and the production summary export. Generic context and
// confined-I/O primitives remain covered by the accepted context dependency.
const WITNESS_SOURCES: &[WitnessSource] = &[
    source!(
        "validator/src/api_witness.rs",
        include_bytes!("../../api_witness.rs"),
        "compiled API declaration"
    ),
    source!(
        "validator/src/lib.rs",
        include_bytes!("../../lib.rs"),
        "root public module export authority"
    ),
    source!(
        "validator/tests/public_api_witness.rs",
        include_bytes!("../../../tests/public_api_witness.rs"),
        "external crate visibility witness"
    ),
    source!(
        "validator/src/command_witness.rs",
        include_bytes!("../../command_witness.rs"),
        "compiled command row digest"
    ),
    source!(
        "validator/src/cli/successor/catalog.rs",
        include_bytes!("../../cli/successor/catalog.rs"),
        "command catalog definition"
    ),
    source!(
        "validator/src/cli/successor/model.rs",
        include_bytes!("../../cli/successor/model.rs"),
        "command group and status model"
    ),
    source!(
        "validator/src/inventory/mod.rs",
        include_bytes!("../mod.rs"),
        "public inventory export"
    ),
    source!(
        "validator/src/inventory/builder.rs",
        include_bytes!("../builder.rs"),
        "guard ordering and catalog finalization"
    ),
    source!(
        "validator/src/inventory/digest.rs",
        include_bytes!("../digest.rs"),
        "activation row digest"
    ),
    source!(
        "validator/src/inventory/fs.rs",
        include_bytes!("../fs.rs"),
        "registry source read gate"
    ),
    source!(
        "validator/src/inventory/projection.rs",
        include_bytes!("../projection.rs"),
        "catalog projection export"
    ),
    source!(
        "validator/src/inventory/registry/command_activation.rs",
        include_bytes!("command_activation.rs"),
        "exact activation guard"
    ),
    source!(
        "validator/src/inventory/registry/data.rs",
        include_bytes!("data.rs"),
        "registry identifier and row storage"
    ),
    source!(
        "validator/src/inventory/registry/integrity.rs",
        include_bytes!("integrity.rs"),
        "adopted registry integrity gate"
    ),
    source!(
        "validator/src/inventory/registry/mod.rs",
        include_bytes!("mod.rs"),
        "registry load and guard export"
    ),
    source!(
        "validator/src/inventory/registry/semantic.rs",
        include_bytes!("semantic.rs"),
        "active API row construction"
    ),
    source!(
        "validator/src/inventory/registry/sources.rs",
        include_bytes!("sources.rs"),
        "registry source coverage gate"
    ),
    source!(
        "validator/src/inventory/registry/topology.rs",
        include_bytes!("topology.rs"),
        "candidate command row construction"
    ),
    source!(
        "validator/src/inventory/types.rs",
        include_bytes!("../types.rs"),
        "active status and catalog serialization"
    ),
    source!(
        "validator/src/inventory/validate/duplicates.rs",
        include_bytes!("../validate/duplicates.rs"),
        "duplicate activation classification"
    ),
    source!(
        "validator/src/inventory/validate.rs",
        include_bytes!("../validate.rs"),
        "catalog status reconciliation"
    ),
    source!(
        "validator/examples/hct_inventory.rs",
        include_bytes!("../../../examples/hct_inventory.rs"),
        "production catalog and closure export"
    ),
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct ActivationRow {
    stable_id: String,
    digest_sha256: String,
    active_status: ActiveStatus,
    references: Vec<String>,
}

impl ActivationRow {
    fn from_entry(entry: &InventoryEntry) -> Self {
        Self {
            stable_id: entry.stable_id.clone(),
            digest_sha256: entry.digest_sha256.clone(),
            active_status: entry.active_status,
            references: entry.references.clone(),
        }
    }
}

fn exact_rows(
    actual: impl IntoIterator<Item = ActivationRow>,
    expected: &BTreeMap<String, ActivationRow>,
) -> Result<(), InventoryError> {
    let mut observed = BTreeMap::new();
    for row in actual {
        if !expected.contains_key(&row.stable_id) {
            return Err(InventoryError::Activation(ActivationFailure::UnknownRow));
        }
        if let Some(existing) = observed.insert(row.stable_id.clone(), row.clone()) {
            return Err(InventoryError::Activation(if existing == row {
                ActivationFailure::DuplicateRow
            } else {
                ActivationFailure::ConflictingRow
            }));
        }
    }
    if observed.len() != expected.len()
        || expected
            .keys()
            .any(|stable_id| !observed.contains_key(stable_id))
    {
        return Err(InventoryError::Activation(ActivationFailure::MissingRow));
    }
    if expected
        .iter()
        .any(|(stable_id, expected)| observed.get(stable_id) != Some(expected))
    {
        return Err(InventoryError::Activation(
            ActivationFailure::ConflictingRow,
        ));
    }
    Ok(())
}

fn exact_set(
    actual: impl IntoIterator<Item = String>,
    expected: &BTreeSet<String>,
) -> Result<(), InventoryError> {
    let mut observed = BTreeSet::new();
    for stable_id in actual {
        if !expected.contains(&stable_id) {
            return Err(InventoryError::Activation(ActivationFailure::UnknownRow));
        }
        if !observed.insert(stable_id) {
            return Err(InventoryError::Activation(ActivationFailure::DuplicateRow));
        }
    }
    if &observed != expected {
        return Err(InventoryError::Activation(ActivationFailure::MissingRow));
    }
    Ok(())
}

fn exact_source_manifest<'a>(
    actual: impl IntoIterator<Item = &'a WitnessSource>,
) -> Result<BTreeMap<&'a str, &'a WitnessSource>, InventoryError> {
    let expected = WITNESS_SOURCES
        .iter()
        .map(|source| source.relative)
        .collect::<BTreeSet<_>>();
    let mut observed = BTreeMap::new();
    for source in actual {
        if !expected.contains(source.relative) {
            return Err(InventoryError::Activation(ActivationFailure::UnknownRow));
        }
        if source.responsibility.trim().is_empty() {
            return Err(InventoryError::Activation(
                ActivationFailure::ConflictingRow,
            ));
        }
        if let Some(existing) = observed.insert(source.relative, source) {
            return Err(InventoryError::Activation(
                if existing.embedded == source.embedded
                    && existing.responsibility == source.responsibility
                {
                    ActivationFailure::DuplicateRow
                } else {
                    ActivationFailure::ConflictingRow
                },
            ));
        }
    }
    if observed.len() != expected.len()
        || expected
            .iter()
            .any(|relative| !observed.contains_key(relative))
    {
        return Err(InventoryError::Activation(ActivationFailure::MissingRow));
    }
    Ok(observed)
}

fn witness_sources_current(reads: &ReadSession, root: &Path) -> Result<bool, InventoryError> {
    let guard_path = root.join("validator/src/inventory/registry/command_activation.rs");
    if fs::symlink_metadata(guard_path).is_err() {
        return Ok(false);
    }
    let sources = exact_source_manifest(WITNESS_SOURCES)?;
    let present = sources
        .keys()
        .filter(|relative| fs::symlink_metadata(root.join(relative)).is_ok())
        .count();
    if present != sources.len() {
        return Err(InventoryError::Activation(ActivationFailure::MissingRow));
    }
    for source in sources.values() {
        let path = root.join(source.relative);
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| InventoryError::Activation(ActivationFailure::MissingRow))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(InventoryError::Activation(ActivationFailure::UnsafeInput));
        }
        let bytes = reads
            .read_bounded(&path, MAX_WITNESS_SOURCE_BYTES)
            .map_err(|error| {
                InventoryError::Activation(
                    if matches!(error, ContextError::ConcurrentMutation(_)) {
                        ActivationFailure::StaleInput
                    } else {
                        ActivationFailure::UnsafeInput
                    },
                )
            })?;
        if bytes.as_slice() != source.embedded {
            return Err(InventoryError::Activation(ActivationFailure::StaleInput));
        }
    }
    Ok(true)
}

pub(crate) fn revalidate_sources(
    reads: &ReadSession,
    root: &Path,
    expected_current: bool,
) -> Result<(), InventoryError> {
    let current = witness_sources_current(reads, root)?;
    if current != expected_current {
        return Err(InventoryError::Activation(ActivationFailure::StaleInput));
    }
    reads
        .revalidate()
        .map_err(|_| InventoryError::Activation(ActivationFailure::StaleInput))
}

fn api_rows() -> Result<BTreeMap<String, ActivationRow>, InventoryError> {
    let mut rows = BTreeMap::new();
    for api in crate::api_witness::implemented_public_apis() {
        let stable_id = format!("API:{api}");
        let row = ActivationRow {
            stable_id: stable_id.clone(),
            digest_sha256: sha256_hex(api.as_bytes()),
            active_status: ActiveStatus::Active,
            references: Vec::new(),
        };
        if rows.insert(stable_id, row).is_some() {
            return Err(InventoryError::Activation(ActivationFailure::DuplicateRow));
        }
    }
    Ok(rows)
}

fn command_rows() -> Result<BTreeMap<String, ActivationRow>, InventoryError> {
    let mut rows = BTreeMap::new();
    for group in crate::command_witness::compiled_groups()
        .map_err(|_| InventoryError::Activation(ActivationFailure::ConflictingRow))?
    {
        let stable_id = format!("COMMAND:{}", group.name);
        let row = ActivationRow {
            stable_id: stable_id.clone(),
            digest_sha256: group.digest_sha256,
            active_status: ActiveStatus::Candidate,
            references: vec![
                "ACTIVATION-CLASS:catalog-definition-only".to_owned(),
                format!("ROUTE-COUNT:{}", group.route_count),
            ],
        };
        if rows.insert(stable_id, row).is_some() {
            return Err(InventoryError::Activation(ActivationFailure::DuplicateRow));
        }
    }
    Ok(rows)
}

pub(crate) fn guard(
    reads: &ReadSession,
    root: &Path,
    registry: &mut RegistryData,
) -> Result<bool, InventoryError> {
    let expected_apis = api_rows()?;
    let expected_commands = command_rows()?;
    let required_apis = registry
        .entries
        .iter()
        .filter(|entry| {
            entry.kind == "source-symbol-implementation"
                && entry.relative_path.starts_with("@semantic/")
        })
        .map(|entry| entry.stable_id.clone())
        .collect::<BTreeSet<_>>();
    if expected_apis
        .keys()
        .any(|stable_id| !required_apis.contains(stable_id))
    {
        return Err(InventoryError::Activation(ActivationFailure::UnknownRow));
    }
    exact_rows(
        registry
            .entries
            .iter()
            .filter(|entry| entry.generator.as_deref() == Some(API_GENERATOR))
            .map(ActivationRow::from_entry),
        &expected_apis,
    )?;
    let required_commands = registry
        .entries
        .iter()
        .filter(|entry| {
            entry.kind == "command-group" && entry.active_status == ActiveStatus::Required
        })
        .map(|entry| entry.stable_id.clone());
    exact_set(
        required_commands,
        &expected_commands.keys().cloned().collect(),
    )?;
    exact_rows(
        registry
            .entries
            .iter()
            .filter(|entry| entry.generator.as_deref() == Some(COMMAND_GENERATOR))
            .map(ActivationRow::from_entry),
        &expected_commands,
    )?;

    let sources_current = witness_sources_current(reads, root)?;
    if !sources_current {
        registry.findings.push(InventoryFinding::warning(
            "activation_witness_source_set_unverified",
            None,
            None,
            "compiled activation witnesses cannot be bound to the target source set".to_owned(),
        ));
    }
    for entry in registry
        .entries
        .iter_mut()
        .filter(|entry| entry.generator.as_deref() == Some(API_GENERATOR))
    {
        if !sources_current {
            entry.active_status = ActiveStatus::Candidate;
        }
        entry.input_provenance.extend(
            WITNESS_SOURCES
                .iter()
                .map(|source| source.relative.to_owned()),
        );
        entry.normalize();
    }
    registry.counts.insert(
        "activation_witness_sources".to_owned(),
        WITNESS_SOURCES.len(),
    );
    registry.counts.insert(
        "verified_api_activations".to_owned(),
        usize::from(sources_current) * expected_apis.len(),
    );
    registry
        .counts
        .insert("verified_command_handler_activations".to_owned(), 0);
    registry.counts.insert(
        "candidate_command_groups".to_owned(),
        expected_commands.len(),
    );
    Ok(sources_current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{BuildRequest, LiveContext};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_REPOSITORY: AtomicU64 = AtomicU64::new(0);

    fn row(id: &str, digest: &str, status: ActiveStatus) -> ActivationRow {
        ActivationRow {
            stable_id: id.to_owned(),
            digest_sha256: digest.to_owned(),
            active_status: status,
            references: Vec::new(),
        }
    }

    #[test]
    fn activation_rows_deny_unknown_missing_duplicate_and_conflict() {
        let expected = BTreeMap::from([(
            "API:Known".to_owned(),
            row("API:Known", "known", ActiveStatus::Active),
        )]);
        for (actual, code) in [
            (
                vec![row("API:Unknown", "known", ActiveStatus::Active)],
                "unknown-row",
            ),
            (Vec::new(), "missing-row"),
            (
                vec![
                    row("API:Known", "known", ActiveStatus::Active),
                    row("API:Known", "known", ActiveStatus::Active),
                ],
                "duplicate-row",
            ),
            (
                vec![row("API:Known", "changed", ActiveStatus::Active)],
                "conflicting-row",
            ),
        ] {
            let error = exact_rows(actual, &expected).unwrap_err();
            assert!(error.to_string().ends_with(code));
        }
    }

    #[test]
    fn command_catalog_rows_are_candidates_not_handler_activations() {
        let rows = command_rows().unwrap();
        assert!(!rows.is_empty());
        assert!(
            rows.values()
                .all(|row| row.active_status == ActiveStatus::Candidate)
        );
    }

    #[test]
    fn activation_source_manifest_rejects_unknown_omitted_duplicate_and_conflicting_rows() {
        static UNKNOWN: WitnessSource = WitnessSource {
            relative: "validator/src/inventory/registry/unknown.rs",
            embedded: b"unknown",
            responsibility: "unknown source",
        };
        let mut unknown = WITNESS_SOURCES.iter().collect::<Vec<_>>();
        unknown.push(&UNKNOWN);
        assert!(
            exact_source_manifest(unknown)
                .unwrap_err()
                .to_string()
                .ends_with("unknown-row")
        );

        assert!(
            exact_source_manifest(WITNESS_SOURCES[..WITNESS_SOURCES.len() - 1].iter())
                .unwrap_err()
                .to_string()
                .ends_with("missing-row")
        );

        let mut duplicate = WITNESS_SOURCES.iter().collect::<Vec<_>>();
        duplicate.push(&WITNESS_SOURCES[0]);
        assert!(
            exact_source_manifest(duplicate)
                .unwrap_err()
                .to_string()
                .ends_with("duplicate-row")
        );

        let conflicting = WitnessSource {
            relative: WITNESS_SOURCES[0].relative,
            embedded: b"changed",
            responsibility: "conflicting responsibility",
        };
        let mut conflict = WITNESS_SOURCES.iter().collect::<Vec<_>>();
        conflict.push(&conflicting);
        assert!(
            exact_source_manifest(conflict)
                .unwrap_err()
                .to_string()
                .ends_with("conflicting-row")
        );
    }

    #[test]
    fn final_source_reread_rejects_semantic_and_root_export_mutate_restore() {
        for (label, relative) in [
            (
                "semantic-mutate-restore",
                "validator/src/inventory/registry/semantic.rs",
            ),
            ("lib-mutate-restore", "validator/src/lib.rs"),
        ] {
            assert_mutate_restore_rejected(label, relative);
        }
    }

    fn assert_mutate_restore_rejected(label: &str, relative: &str) {
        let root = source_repository(label);
        let context = LiveContext::build(BuildRequest::new(&root)).unwrap();
        let reads = context.begin_read_session().unwrap();
        assert!(witness_sources_current(&reads, &root).unwrap());

        let target = root.join(relative);
        let original = fs::read(&target).unwrap();
        let mut changed = original.clone();
        changed[0] ^= 1;
        fs::write(&target, changed).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        fs::write(&target, &original).unwrap();

        assert!(
            revalidate_sources(&reads, &root, true)
                .unwrap_err()
                .to_string()
                .ends_with("stale-input")
        );
        drop(reads);
        drop(context);
        fs::remove_dir_all(root).unwrap();
    }

    fn source_repository(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "ultragoal-activation-{label}-{}-{}",
            std::process::id(),
            NEXT_REPOSITORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let root = fs::canonicalize(root).unwrap();
        git(&root, &["init", "-q"]);
        git(
            &root,
            &["config", "user.email", "activation@example.invalid"],
        );
        git(&root, &["config", "user.name", "Activation Test"]);
        for source in WITNESS_SOURCES {
            let path = root.join(source.relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, source.embedded).unwrap();
        }
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-q", "-m", "fixture"]);
        root
    }

    fn git(root: &Path, arguments: &[&str]) {
        assert!(
            Command::new("git")
                .args(arguments)
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
    }
}
