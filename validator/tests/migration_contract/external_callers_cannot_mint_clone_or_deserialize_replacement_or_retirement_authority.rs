#[test]
fn external_callers_cannot_mint_clone_or_deserialize_replacement_or_retirement_authority() {
    let root = temp_root("public-retirement-seal");
    fs::create_dir_all(root.join("src/bin")).unwrap();
    fs::create_dir_all(root.join("src/migration/product")).unwrap();
    fs::create_dir_all(root.join("src/migration/product/host")).unwrap();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/migration/mod.rs"),
        root.join("src/migration.rs"),
    )
    .unwrap();
    for name in ["mod.rs", "model.rs", "registry.rs", "runtime.rs"] {
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/migration/product")
                .join(name),
            root.join("src/migration/product").join(name),
        )
        .unwrap();
    }
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/migration/product/host.rs"),
        root.join("src/migration/product/host.rs"),
    )
    .unwrap();
    for name in [
        "authority.rs",
        "effects.rs",
        "filesystem.rs",
        "source.rs",
        "store.rs",
        "test_support.rs",
    ] {
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/migration/product/host")
                .join(name),
            root.join("src/migration/product/host").join(name),
        )
        .unwrap();
    }
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "migration-retirement-seal-probe"
version = "0.0.0"
edition = "2024"

[dependencies]
getrandom = { version = "=0.4.3", default-features = false }
hmac = { version = "=0.12.1", default-features = false }
libc = "0.2.186"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sha2 = "0.10"
"#,
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "pub mod migration;\n").unwrap();
    fs::write(
        root.join("src/bin/forge.rs"),
        r#"use migration_retirement_seal_probe::migration::{
    DestructiveAuthorization, MigrationPlan, MigrationPlanProjection, ReplacementEvidence,
    RetirementReview, RetirementTarget,
};

fn forge_replacement() {
    let _ = ReplacementEvidence::new(
        "LEGACY-SKILL:old",
        "SKILL:current",
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    let _ = MigrationPlan::build;
}

fn forge_review() {
    let _ = RetirementReview::new(
        "caller-reviewer",
        true,
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        true,
        true,
    );
}

fn forge_effect() {
    let _ = DestructiveAuthorization::new();
}

fn clone_review(value: RetirementReview) {
    let _ = value.clone();
}

fn clone_replacement(value: ReplacementEvidence) {
    let _ = value.clone();
}

fn clone_plan(value: MigrationPlan) {
    let _ = value.clone();
}

fn clone_target(value: RetirementTarget) {
    let _ = value.clone();
}

fn require_serializable<T: serde::Serialize>(_: &T) {}
fn require_deserializable<T: for<'de> serde::Deserialize<'de>>() {}

fn deserialize_review() {
    require_deserializable::<ReplacementEvidence>();
    require_deserializable::<RetirementReview>();
    require_deserializable::<DestructiveAuthorization>();
    require_deserializable::<MigrationPlan>();
    require_deserializable::<RetirementTarget>();
}

fn serialize_plan(value: &MigrationPlan) {
    require_serializable(value);
}

fn serialize_target(value: &RetirementTarget) {
    require_serializable(value);
}

fn replay_projection(value: MigrationPlanProjection) {
    let _plan: MigrationPlan = value.into();
}

fn main() {}
"#,
    )
    .unwrap();
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["check", "--offline", "--quiet", "--bin", "forge"])
        .env("CARGO_TARGET_DIR", root.join("target"))
        .current_dir(&root)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "external forgery unexpectedly compiled"
    );
    for expected in [
        "RetirementReview",
        "DestructiveAuthorization",
        "ReplacementEvidence",
        "MigrationPlan",
        "RetirementTarget",
        "build",
        "new",
        "clone",
        "Serialize",
        "Deserialize",
        "From<MigrationPlanProjection>",
    ] {
        assert!(
            stderr.contains(expected),
            "unexpected compile failure: {stderr}"
        );
    }

    let migration_source = root.join("src/migration.rs");
    let mut source = fs::read_to_string(&migration_source).unwrap();
    source.push_str(
        r#"
fn consumed_replacement_trait_probe(value: ConsumedReplacementEvidence) {
    fn require_serializable<T: serde::Serialize>(_: &T) {}
    fn require_deserializable<T: for<'de> serde::Deserialize<'de>>() {}
    let _ = value.clone();
    require_serializable(&value);
    require_deserializable::<ConsumedReplacementEvidence>();
}
"#,
    );
    fs::write(&migration_source, source).unwrap();
    let consumed_output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["check", "--offline", "--quiet", "--lib"])
        .env("CARGO_TARGET_DIR", root.join("target-consumed"))
        .current_dir(&root)
        .output()
        .unwrap();
    let consumed_stderr = String::from_utf8_lossy(&consumed_output.stderr);
    assert!(
        !consumed_output.status.success(),
        "consumed replacement evidence unexpectedly implemented copy/serde traits"
    );
    for expected in [
        "ConsumedReplacementEvidence",
        "clone",
        "Serialize",
        "Deserialize",
    ] {
        assert!(
            consumed_stderr.contains(expected),
            "unexpected consumed-token compile failure: {consumed_stderr}"
        );
    }
    fs::remove_dir_all(root).unwrap();
}
