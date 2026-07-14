use super::*;
use crate::distribution::host_capability::{HostCapabilityDeclaration, JourneyBinding};
use std::path::PathBuf;

const A: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const C: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const D: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

fn package() -> PackageIdentity {
    PackageIdentity::new(
        SourceIdentity::new(
            A.into(),
            B.into(),
            "harness-ultragoal".into(),
            "0.0.12".into(),
            C.into(),
            C.into(),
        )
        .unwrap(),
        C.into(),
        D.into(),
    )
    .unwrap()
}

fn binding(package: &PackageIdentity, label: &str) -> (JourneyBinding, PathBuf) {
    let root = std::env::temp_dir().join(format!("hul-{label}-{}", std::process::id()));
    let home = root.join("home");
    let project = root.join("project");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&project).unwrap();
    let host = HostCapabilityDeclaration::isolated(&home, &project, label, None).unwrap();
    (
        JourneyBinding::new(package.clone(), &host, "local-harness-plugins").unwrap(),
        root,
    )
}

#[test]
fn same_package_rows_from_mixed_journeys_fail_every_canonical_verifier() {
    let package = package();
    let (first, first_root) = binding(&package, "mixed-journey-first");
    let (second, second_root) = binding(&package, "mixed-journey-second");
    assert_ne!(first.binding_sha256(), second.binding_sha256());
    let rows = IdentitySurface::ALL.map(|surface| {
        let observed_tree = matches!(surface, IdentitySurface::Installed | IdentitySurface::Cache)
            .then(|| package.tree_sha256().into());
        SurfaceIdentity::issue(package.clone(), surface, A.into(), observed_tree)
            .and_then(|row| {
                row.bind_journey(if surface == IdentitySurface::AppRegistry {
                    &second
                } else {
                    &first
                })
            })
            .unwrap()
    });
    assert_eq!(
        verify_surface_chain(&rows).unwrap_err().id(),
        DistributionErrorId::ProvenanceMismatch
    );
    for binding in [&first, &second] {
        assert_eq!(
            verify_bound_surface_chain(&rows, binding).unwrap_err().id(),
            DistributionErrorId::ProvenanceMismatch
        );
    }
    assert_eq!(
        crate::distribution::verify_identity_ladder(&serde_json::to_vec(&rows).unwrap())
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch
    );
    std::fs::remove_dir_all(first_root).unwrap();
    std::fs::remove_dir_all(second_root).unwrap();
}
