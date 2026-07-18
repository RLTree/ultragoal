use crate::distribution::{ConfinedRoot, TreeObject, TreeObjectKind};
use crate::host_lifecycle::{
    DarwinHostDiagnosis, DarwinHostErrorId, DarwinHostSurface, DarwinHostTransactionAdapter,
    DarwinSurfaceStatus,
};
use crate::transaction_fixture::{FOREIGN_CANDIDATE, Fixture, MARKETPLACE};
use serde::Deserialize;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::os::unix::net::UnixListener;

mod lineage_corruption {
    include!("lineage_corruption.rs");
}
mod package_and_journal_substitution {
    include!("package_and_journal_substitution.rs");
}
mod terminal_predecessor_binding {
    include!("terminal_predecessor_binding.rs");
}
mod unsafe_surface_objects {
    include!("unsafe_surface_objects.rs");
}

fn replace_once(text: &str, from: &str, to: &str) -> String {
    assert_eq!(text.matches(from).count(), 1, "expected one {from}");
    text.replacen(from, to, 1)
}

fn replace_after(text: &str, marker: &str, from: &str, to: &str) -> String {
    let offset = text.find(marker).expect("lineage marker") + marker.len();
    let (prefix, suffix) = text.split_at(offset);
    assert!(suffix.contains(from), "expected {from} after {marker}");
    format!("{prefix}{}", suffix.replacen(from, to, 1))
}

fn installed_fixture(label: &str) -> Fixture {
    let fixture = Fixture::new(label);
    let package = fixture.bundle("0.0.12");
    fixture
        .adapter
        .execute(&fixture.install_plan(&package))
        .unwrap();
    fixture
}

fn installed_plan(fixture: &Fixture) -> crate::host_lifecycle::DarwinHostTransactionPlan {
    let package = fixture.bundle("0.0.12");
    fixture.reinstall_plan(&package)
}

fn assert_unsafe(fixture: &Fixture, surface: DarwinHostSurface) {
    let plan = installed_plan(fixture);
    let error = fixture.adapter.query(&plan).unwrap_err();
    assert_eq!(error.id(), DarwinHostErrorId::UnsafeObject);
    assert_eq!(error.surface(), Some(surface));
}

fn replace_all_surfaces(fixture: &Fixture, package: &crate::transaction_fixture::Bundle) {
    for surface in DarwinHostSurface::ALL {
        fixture
            .adapter
            .replace_surface_for_test(surface, &package.snapshot)
            .unwrap();
    }
}
