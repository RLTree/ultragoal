#[test]
fn independent_product_captures_are_byte_identical_and_reverified() {
    let repo = Repo::new("supported-package-product-identical");
    let context = repo.context();
    let catalog = catalog(&context);
    let before = status(&repo.root);
    let before_tree = source_tree(&repo.root);

    let first = capture_product_package(&context, &catalog).expect("first package");
    let second = capture_product_package(&context, &catalog).expect("second package");
    assert_eq!(first.snapshot().archive(), second.snapshot().archive());
    assert_eq!(first.snapshot().inventory(), second.snapshot().inventory());
    assert_eq!(first.source_inventory(), second.source_inventory());
    assert_eq!(first.snapshot().entries().len(), 18);
    assert_eq!(
        first
            .snapshot()
            .entries()
            .iter()
            .filter(|entry| entry.role() == PackageRole::Skill)
            .count(),
        8
    );
    for role in [PackageRole::Documentation, PackageRole::Data] {
        assert_eq!(
            first
                .snapshot()
                .entries()
                .iter()
                .filter(|entry| entry.role() == role)
                .count(),
            0,
            "noncanonical package role became active: {role:?}"
        );
    }
    assert_eq!(
        first
            .snapshot()
            .entries()
            .iter()
            .filter(|entry| entry.role() == PackageRole::Executable)
            .count(),
        1
    );
    assert_eq!(
        first
            .snapshot()
            .entries()
            .iter()
            .filter(|entry| entry.role() == PackageRole::Agent)
            .count(),
        8
    );
    verify_product_package(&first, &context, &catalog).expect("independent verify");
    assert_eq!(status(&repo.root), before);
    assert_eq!(source_tree(&repo.root), before_tree);
}

#[test]
fn legacy_skill_source_is_not_active_package_membership() {
    let repo = Repo::new("supported-package-product-legacy-excluded");
    fs::create_dir_all(repo.root.join("skills/legacy/agents")).expect("legacy metadata");
    fs::write(repo.root.join("skills/legacy/SKILL.md"), "legacy\n").expect("legacy skill");
    fs::write(
        repo.root.join("skills/legacy/agents/openai.yaml"),
        "interface: {}\n",
    )
    .expect("legacy metadata");
    let context = repo.context();
    let catalog = catalog(&context);
    let package = capture_product_package(&context, &catalog).expect("package");
    assert_eq!(package.snapshot().entries().len(), 18);
    assert!(
        package
            .snapshot()
            .entries()
            .iter()
            .all(|entry| !entry.path().starts_with("skills/legacy/"))
    );
}

#[test]
fn captured_runtime_payload_drives_the_confined_runtime_probe() {
    let repo = Repo::new("supported-package-product-runtime");
    let context = repo.context();
    let artifact = capture_product_package(&context, &catalog(&context)).expect("package");
    let output = OutputRoot::new("supported-package-product-runtime");
    let confined = ConfinedRoot::open(&output.root).expect("confined root");
    let executable = ScopedFile::new(confined.clone(), "runtime/runtime-probe-bin").unwrap();
    publish_installed_runtime_probe(artifact.snapshot(), &executable).expect("runtime payload");
    let program = output.root.join("runtime/runtime-probe-bin");
    let host = HostCapabilityDeclaration::isolated(
        &output.root,
        &output.root,
        "isolated-runtime-v1",
        Some(&program),
    )
    .expect("runtime host");
    let binding = JourneyBinding::new(
        artifact.snapshot().identity().clone(),
        &host,
        "local-harness-plugins",
    )
    .expect("journey binding");
    let plan = InstallPlan::new(
        artifact.snapshot().context_id().into(),
        artifact.snapshot().candidate_id().into(),
        InstallScope::PersonalFixture,
        "plugins/harness-ultragoal.hugpkg".into(),
        artifact.snapshot().package_sha256().into(),
        ExpectedPrior::Absent,
    )
    .expect("install plan");
    let mut install_effects = ScopedInstall::new(confined.clone());
    let mut installed = install(&plan, artifact.snapshot(), &mut install_effects).expect("install");
    installed.bind_journey(&binding).expect("bound install");
    let runtime = RuntimeProbePlan::from_installed_package(InstalledPackageRuntimeProbeRequest {
        binding,
        host: &host,
        install: installed.snapshot(),
        effects: &mut ScopedInstall::new(confined),
        package: artifact.snapshot(),
        program: &program,
        argv: vec!["valid".into()],
        timeout: Duration::from_secs(10),
    })
    .expect("runtime plan")
    .execute_bound()
    .expect("runtime execution");
    assert_eq!(runtime.0.runtime_verdict(), RuntimeVerdict::Executed);
}

#[test]
fn verified_artifact_archive_publication_is_repeatable() {
    let repo = Repo::new("supported-package-product-archive");
    let context = repo.context();
    let catalog = catalog(&context);
    let artifact = capture_product_package(&context, &catalog).expect("package");
    let output = OutputRoot::new("supported-package-product-archive");
    let file = ScopedFile::new(
        ConfinedRoot::open(&output.root).expect("confined root"),
        "packages/harness-ultragoal.hugpkg",
    )
    .expect("archive output");
    artifact
        .publish_archive(&context, &catalog, &file)
        .expect("first publication");
    artifact
        .publish_archive(&context, &catalog, &file)
        .expect("repeat publication");
    assert_eq!(
        file.inspect(65 * 1024 * 1024).unwrap().unwrap(),
        artifact.snapshot().archive()
    );
}
use crate::distribution::{
    ExpectedPrior, InstallPlan, InstallScope, InstalledPackageRuntimeProbeRequest,
    RuntimeProbePlan, RuntimeVerdict, ScopedInstall, install, publish_installed_runtime_probe,
};
use std::time::Duration;
