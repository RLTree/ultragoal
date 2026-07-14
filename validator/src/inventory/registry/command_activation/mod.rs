use super::RegistryData;
use crate::context::{ContextError, ReadSession};
use crate::inventory::digest::sha256_hex;
use crate::inventory::types::{
    ActivationFailure, ActiveStatus, InventoryEntry, InventoryError, InventoryFinding,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

include!("max_witness_source_bytes.rs");

macro_rules! source {
    ($relative:literal, $embedded:expr, $responsibility:literal) => {
        WitnessSource {
            relative: $relative,
            embedded: $embedded,
            responsibility: $responsibility,
        }
    };
}

include!("witness_sources.rs");

include!("exact_source_manifest.rs");

include!("guard.rs");

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
