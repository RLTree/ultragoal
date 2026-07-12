#![cfg(unix)]

use crate::distribution::{
    DistributionErrorId as ErrorId, ExpectedTree, MaterializeEffects, ScopedTree, TreeObject,
    tree_sha256,
};
use crate::distribution::{
    PackageArtifactBinding, publish_package_artifact, reconcile_package_artifact,
    recover_package_artifact, rollback_package_artifact,
};
use crate::journey_support::JourneyFixture;
use crate::support::digest;
use std::fs;

#[derive(Default)]
struct CorruptAfterWrite {
    rows: Option<Vec<TreeObject>>,
    reads: usize,
    corrupt_on_read: usize,
}

#[derive(Default)]
struct EffectCounter {
    reads: usize,
    transitions: usize,
}

impl MaterializeEffects for EffectCounter {
    fn read_tree(&mut self, _: usize, _: usize) -> Result<Option<Vec<TreeObject>>, ()> {
        self.reads += 1;
        Ok(None)
    }

    fn compare_exchange_tree(
        &mut self,
        _: Option<&str>,
        _: Option<&[TreeObject]>,
    ) -> Result<bool, ()> {
        self.transitions += 1;
        Ok(true)
    }
}

impl MaterializeEffects for CorruptAfterWrite {
    fn read_tree(&mut self, _: usize, _: usize) -> Result<Option<Vec<TreeObject>>, ()> {
        self.reads += 1;
        let mut rows = self.rows.clone();
        if self.reads == self.corrupt_on_read {
            if let Some(row) = rows.as_mut().and_then(|rows| rows.first_mut()) {
                *row = TreeObject::regular(row.path().into(), 0o644, b"substitute".to_vec());
            }
        }
        Ok(rows)
    }

    fn compare_exchange_tree(
        &mut self,
        expected: Option<&str>,
        replacement: Option<&[TreeObject]>,
    ) -> Result<bool, ()> {
        if self
            .rows
            .as_deref()
            .map(tree_sha256)
            .transpose()
            .map_err(|_| ())?
            .as_deref()
            != expected
        {
            return Ok(false);
        }
        self.rows = replacement.map(<[TreeObject]>::to_vec);
        Ok(true)
    }
}

fn binding(snapshot: &crate::distribution::PackageSnapshot) -> PackageArtifactBinding {
    PackageArtifactBinding::issue(snapshot).unwrap()
}

#[test]
fn writes_one_verified_candidate_bound_pair_and_reproduces_cleanly() {
    let first = JourneyFixture::new("package-output-first");
    let second = JourneyFixture::new("package-output-second");
    let one = first.build("build/one.hugpkg");
    let two = second.build("build/two.hugpkg");
    assert_eq!(one.archive(), two.archive());
    assert_eq!(one.inventory(), two.inventory());

    let mut left = ScopedTree::new(first.confined(), "output/candidate").unwrap();
    let mut right = ScopedTree::new(second.confined(), "output/candidate").unwrap();
    let left_tx =
        publish_package_artifact(&one, &binding(&one), &ExpectedTree::Absent, &mut left).unwrap();
    let right_tx =
        publish_package_artifact(&two, &binding(&two), &ExpectedTree::Absent, &mut right).unwrap();
    let left_rows = left.inspect(2, 65 * 1024 * 1024).unwrap().unwrap();
    let right_rows = right.inspect(2, 65 * 1024 * 1024).unwrap().unwrap();
    assert_eq!(left_rows, right_rows);
    assert_eq!(left_tx.output_tree_sha256(), right_tx.output_tree_sha256());
    reconcile_package_artifact(&one, &binding(&one), Some(&left_rows)).unwrap();
    assert_eq!(left_rows.len(), 2);
    assert!(left_rows.iter().any(|row| row.path().ends_with(".hugpkg")));
    assert!(
        left_rows
            .iter()
            .any(|row| row.path().ends_with(".inventory.json"))
    );
}

#[test]
fn mismatched_binding_partial_pair_and_extra_or_executable_rows_fail_without_writes() {
    let fixture = JourneyFixture::new("package-output-negative");
    let snapshot = fixture.build("build/one.hugpkg");
    let output = ScopedTree::new(fixture.confined(), "output/candidate").unwrap();
    let wrong = binding(&snapshot).substituted("context_id", &digest(b"wrong-context"));
    assert_eq!(
        reconcile_package_artifact(&snapshot, &wrong, None)
            .unwrap_err()
            .id(),
        ErrorId::ProvenanceMismatch
    );
    assert!(output.inspect(2, 65 * 1024 * 1024).unwrap().is_none());

    let mut rows = vec![crate::distribution::TreeObject::regular(
        "harness-ultragoal-0.0.11.hugpkg".into(),
        0o644,
        snapshot.archive().to_vec(),
    )];
    assert_eq!(
        reconcile_package_artifact(&snapshot, &binding(&snapshot), Some(&rows))
            .unwrap_err()
            .id(),
        ErrorId::ArchiveMismatch
    );
    rows.push(crate::distribution::TreeObject::regular(
        "unexpected.bin".into(),
        0o755,
        b"executable".to_vec(),
    ));
    assert_eq!(
        reconcile_package_artifact(&snapshot, &binding(&snapshot), Some(&rows))
            .unwrap_err()
            .id(),
        ErrorId::ArchiveMismatch
    );
}

#[test]
fn every_identity_dimension_is_checked_before_any_effect() {
    let fixture = JourneyFixture::new("package-output-full-identity");
    let snapshot = fixture.build("build/one.hugpkg");
    let permit = binding(&snapshot);
    for (dimension, replacement) in [
        ("context_id", digest(b"wrong-context")),
        ("candidate_id", digest(b"wrong-candidate")),
        ("plugin_id", "wrong-plugin".into()),
        ("version", "0.0.12".into()),
        ("catalog_id", digest(b"wrong-catalog")),
        (
            "accepted_inventory_sha256",
            digest(b"wrong-accepted-inventory"),
        ),
        ("source_tree_sha256", digest(b"wrong-source-tree")),
        ("package_sha256", digest(b"wrong-package")),
        ("inventory_sha256", digest(b"wrong-inventory")),
    ] {
        let mut effects = EffectCounter::default();
        let altered = permit.substituted(dimension, &replacement);
        assert_eq!(
            publish_package_artifact(&snapshot, &altered, &ExpectedTree::Absent, &mut effects)
                .unwrap_err()
                .id(),
            ErrorId::ProvenanceMismatch,
            "{dimension}"
        );
        assert_eq!((effects.reads, effects.transitions), (0, 0), "{dimension}");
    }
}

#[test]
fn verification_mismatch_rolls_back_the_exact_prior_pair() {
    let fixture = JourneyFixture::new("package-output-rollback");
    let snapshot = fixture.build("build/one.hugpkg");
    let binding = binding(&snapshot);
    let mut effects = CorruptAfterWrite::default();
    let prior =
        publish_package_artifact(&snapshot, &binding, &ExpectedTree::Absent, &mut effects).unwrap();
    let before = effects.rows.clone();
    effects.corrupt_on_read = effects.reads + 2;
    assert_eq!(
        publish_package_artifact(
            &snapshot,
            &binding,
            &ExpectedTree::ExactDigest(prior.output_tree_sha256().into()),
            &mut effects,
        )
        .unwrap_err()
        .id(),
        ErrorId::ArchiveMismatch
    );
    assert_eq!(effects.rows, before);
}

#[test]
fn rollback_and_descriptor_recovery_preserve_the_confined_output_pair() {
    let fixture = JourneyFixture::new("package-output-recovery");
    let snapshot = fixture.build("build/one.hugpkg");
    let binding = binding(&snapshot);
    let mut output = ScopedTree::new(fixture.confined(), "output/candidate").unwrap();
    let transaction =
        publish_package_artifact(&snapshot, &binding, &ExpectedTree::Absent, &mut output).unwrap();
    rollback_package_artifact(transaction, &mut output).unwrap();
    assert!(output.inspect(2, 65 * 1024 * 1024).unwrap().is_none());

    let token = &digest(b"output/candidate")[7..];
    let backup = fixture
        .root
        .join(format!(".hul-tree-{token}-backup-interrupted"));
    fs::create_dir(&backup).unwrap();
    fs::write(
        backup.join("harness-ultragoal-0.0.11.hugpkg"),
        snapshot.archive(),
    )
    .unwrap();
    fs::write(
        backup.join("harness-ultragoal-0.0.11.inventory.json"),
        snapshot.inventory(),
    )
    .unwrap();
    assert!(recover_package_artifact(&output).unwrap());
    let recovered = output.inspect(2, 65 * 1024 * 1024).unwrap().unwrap();
    reconcile_package_artifact(&snapshot, &binding, Some(&recovered)).unwrap();
}
