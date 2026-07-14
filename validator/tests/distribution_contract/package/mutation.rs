enum Mutation {
    Bytes(Vec<u8>),
    Special,
}

#[derive(Default)]
struct Sink {
    bytes: Option<Vec<u8>>,
    special: bool,
    reads: usize,
    transitions: usize,
    writes: usize,
    corrupt_read_at: Option<usize>,
    fail_read_at: Option<usize>,
    fail_transition_at: Option<usize>,
    mutate_before_transition: Option<(usize, Mutation)>,
}

impl PackageEffects for Sink {
    fn read_package(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.reads += 1;
        if self.fail_read_at == Some(self.reads) {
            return Err(());
        }
        let mut bytes = self.bytes.clone();
        if self.corrupt_read_at == Some(self.reads)
            && let Some(value) = bytes.as_mut().filter(|value| !value.is_empty())
        {
            value[0] ^= 1;
        }
        Ok(if self.special { None } else { bytes })
    }

    fn compare_exchange_package(
        &mut self,
        expected_sha256: Option<&str>,
        replacement: Option<&[u8]>,
    ) -> Result<bool, ()> {
        self.transitions += 1;
        if self
            .mutate_before_transition
            .as_ref()
            .is_some_and(|(at, _)| *at == self.transitions)
        {
            match self.mutate_before_transition.take().unwrap().1 {
                Mutation::Bytes(bytes) => {
                    self.bytes = Some(bytes);
                    self.special = false;
                }
                Mutation::Special => {
                    self.bytes = None;
                    self.special = true;
                }
            }
        }
        if self.fail_transition_at == Some(self.transitions) {
            return Err(());
        }
        let current_sha256 = self.bytes.as_deref().map(digest);
        if self.special || current_sha256.as_deref() != expected_sha256 {
            return Ok(false);
        }
        if self.bytes.as_deref() != replacement {
            self.writes += 1;
            self.bytes = replacement.map(<[u8]>::to_vec);
        }
        Ok(true)
    }
}

#[test]
fn deterministic_plan_build_inventory_and_independent_verify_match() {
    let fixture = Fixture::complete("package-deterministic");
    write_sources(&fixture);
    let before = tree(&fixture.root);
    let first_plan = plan_package(&fixture.root, &spec(false)).expect("first plan");
    let second_plan = plan_package(&fixture.root, &spec(true)).expect("second plan");
    assert_eq!(
        tree(&fixture.root),
        before,
        "package planning must be zero-write"
    );
    let mut first_sink = Sink::default();
    let mut second_sink = Sink::default();
    let first = build_package(&first_plan, &mut first_sink).expect("first package");
    let second = build_package(&second_plan, &mut second_sink).expect("second package");

    assert_eq!(first.archive(), second.archive());
    assert_eq!(first.package_sha256(), second.package_sha256());
    assert_eq!(first.inventory(), second.inventory());
    assert_eq!(first.entries().len(), 3);
    assert_eq!(first.entries()[0].path(), ".codex-plugin/plugin.json");
    assert_eq!(first.entries()[0].mode(), 0o644);
    assert_eq!(
        first.entries()[1].path(),
        "skills/harness-ultragoal/SKILL.md"
    );
    assert_eq!(first.entries()[1].mode(), 0o644);
    let debug = format!("{first:?}");
    assert!(!debug.contains("Harness front door"));
    assert!(!debug.contains("Proof workflow"));
    assert_eq!(
        verify_package(&first_plan, first.archive())
            .expect("independent verify")
            .package_sha256(),
        first.package_sha256()
    );
}

#[test]
fn package_output_compare_exchange_and_conditional_rollback_fail_closed() {
    let fixture = Fixture::complete("package-substitution");
    write_sources(&fixture);
    let plan = plan_package(&fixture.root, &spec(false)).expect("plan");
    let mut good = Sink::default();
    let snapshot = build_package(&plan, &mut good).expect("package");
    let archive = snapshot.archive().to_vec();
    let mut changed = snapshot.archive().to_vec();
    changed.push(0);
    let changed_error = verify_package(&plan, &changed).unwrap_err().id();
    assert_eq!(changed_error, ErrorId::ArchiveMismatch);
    for prior in [None, Some(b"exact prior".to_vec())] {
        let concurrent = b"SECRET concurrent substitution".to_vec();
        let mut sink = Sink {
            bytes: prior,
            mutate_before_transition: Some((1, Mutation::Bytes(concurrent.clone()))),
            ..Sink::default()
        };
        let failure = build_package(&plan, &mut sink).unwrap_err();
        assert_eq!(failure.id(), ErrorId::InstallConflict);
        assert_eq!(sink.bytes, Some(concurrent));
        assert_eq!(sink.writes, 0);
        assert!(!failure.to_string().contains("SECRET"));
    }
    let prior = b"prior package output".to_vec();
    let mut write = Sink::default();
    write.bytes = Some(prior.clone());
    write.fail_transition_at = Some(1);
    assert_eq!(build_error(&plan, &mut write), ErrorId::EffectFailed);
    assert_eq!(write.bytes, Some(prior.clone()));
    let mut verify = Sink::default();
    verify.bytes = Some(prior.clone());
    verify.corrupt_read_at = Some(2);
    assert_eq!(build_error(&plan, &mut verify), ErrorId::ArchiveMismatch);
    assert_eq!(verify.bytes, Some(prior));

    let concurrent = b"user mutation before rollback".to_vec();
    let mut conflict = Sink::default();
    conflict.fail_read_at = Some(2);
    conflict.mutate_before_transition = Some((2, Mutation::Bytes(concurrent.clone())));
    assert_eq!(build_error(&plan, &mut conflict), ErrorId::InstallConflict);
    assert_eq!(conflict.bytes, Some(concurrent));

    let mut rollback = Sink::default();
    rollback.fail_read_at = Some(2);
    rollback.fail_transition_at = Some(2);
    assert_eq!(build_error(&plan, &mut rollback), ErrorId::RollbackFailed);
    assert_eq!(rollback.bytes, Some(archive.clone()));

    let mut special = Sink::default();
    special.mutate_before_transition = Some((1, Mutation::Special));
    assert_eq!(build_error(&plan, &mut special), ErrorId::InstallConflict);
    assert!(special.special);

    let mut idempotent = Sink::default();
    idempotent.bytes = Some(archive);
    build_package(&plan, &mut idempotent).expect("idempotent build");
    assert_eq!((idempotent.writes, idempotent.transitions), (0, 1));
}

fn build_error(plan: &PackagePlan, sink: &mut Sink) -> ErrorId {
    build_package(plan, sink).unwrap_err().id()
}
