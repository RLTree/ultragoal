mod claim;
mod cleanup_hook;
mod construction;
mod custody;
mod custody_types;
mod directory_entries;
mod invocation;
mod invocation_controls;
mod lifecycle;
mod quarantine;
mod rebind;
mod scope;

use scope::ClaimedFixtureScope;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

pub(crate) const VALID_CATALOG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/routine-production-catalog/valid-catalog-v2.json"
));

static NEXT: AtomicU64 = AtomicU64::new(0);
static FIXTURE_ROOT_LOCK: Mutex<()> = Mutex::new(());

type FixtureRootGuard = MutexGuard<'static, ()>;

fn lock_fixture_root() -> FixtureRootGuard {
    FIXTURE_ROOT_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

pub(crate) struct TestRoot {
    pub(crate) path: PathBuf,
    scope: ClaimedFixtureScope,
}

impl TestRoot {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn write_catalog(&self, bytes: &[u8]) {
        fs::write(self.path.join("config/routines.json"), bytes).unwrap();
    }

    pub(crate) fn teardown_after_assertions(&mut self) {
        self.scope
            .teardown_after_assertions()
            .expect("catalog fixture descriptor-held teardown failed");
        assert!(
            !self.path.exists(),
            "catalog fixture teardown retained scope: {}",
            self.path.display()
        );
    }
}

pub(crate) use invocation::run_catalog_case;

pub(crate) fn verify_catalog_scope_construction() {
    construction::verify_catalog_scope_construction();
}

#[test]
fn fixture_module_tree_is_exclusive() {
    let fixture = include_str!("mod.rs");
    let harness = include_str!("../../mod.rs");
    for child in [
        "claim",
        "cleanup_hook",
        "construction",
        "custody",
        "custody_types",
        "directory_entries",
        "invocation",
        "invocation_controls",
        "lifecycle",
        "quarantine",
        "rebind",
        "scope",
    ] {
        assert!(
            fixture.contains(&format!("mod {child};")),
            "missing {child}"
        );
        assert_eq!(harness.matches("mod catalog_fixture;").count(), 1);
        assert!(
            !harness.contains(&format!("mod {child};")),
            "catalog test root mounted fixture child {child}"
        );
    }
}
