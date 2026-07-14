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
fn digest_matched_partial_pair_is_not_completed_or_replaced() {
    let fixture = JourneyFixture::new("package-output-partial-pair");
    let snapshot = fixture.build("build/one.hugpkg");
    let partial = vec![TreeObject::regular(
        "harness-ultragoal-0.0.11.hugpkg".into(),
        0o644,
        snapshot.archive().to_vec(),
    )];
    let partial_sha256 = tree_sha256(&partial).unwrap();
    let mut effects = CorruptAfterWrite {
        rows: Some(partial.clone()),
        ..CorruptAfterWrite::default()
    };

    assert_eq!(
        publish_package_artifact(
            &snapshot,
            &binding(&snapshot),
            &ExpectedTree::ExactDigest(partial_sha256),
            &mut effects,
        )
        .unwrap_err()
        .id(),
        ErrorId::InstallConflict
    );
    assert_eq!(effects.rows, Some(partial));
    assert_eq!(effects.reads, 1);
}
