use super::*;

#[test]
fn malformed_unknown_and_substituted_lineage_records_fail_closed() {
    let fixture = Fixture::new("lineage-input-rejection");
    let package = fixture.bundle("0.0.12");
    let plan = fixture.install_plan(&package);
    fixture.adapter.execute(&plan).unwrap();
    let path = fixture.lineage_record_path();
    let canonical = fs::read(&path).unwrap();
    let canonical_text = String::from_utf8(canonical.clone()).unwrap();
    let foreign_root = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    let surfaces =
        DarwinHostSurface::ALL.map(|surface| fs::read(fixture.record_path(surface)).unwrap());
    let mut unknown = canonical.clone();
    unknown.pop();
    unknown.extend_from_slice(b",\"unknown\":true}");

    for replacement in [
        b"not-json".to_vec(),
        unknown,
        canonical_text
            .replace(fixture.adapter.root_id(), foreign_root)
            .into_bytes(),
        canonical_text
            .replace("\"generation\":1", "\"generation\":0")
            .into_bytes(),
    ] {
        fs::write(&path, replacement).unwrap();
        assert_eq!(
            fixture.adapter.query(&plan).unwrap_err().id(),
            DarwinHostErrorId::LineageCorrupt
        );
        for (surface, bytes) in DarwinHostSurface::ALL.into_iter().zip(surfaces.clone()) {
            assert_eq!(fs::read(fixture.record_path(surface)).unwrap(), bytes);
        }
        fs::write(&path, &canonical).unwrap();
    }
}

#[test]
fn fresh_reopen_rejects_each_terminal_lineage_identity_substitution_without_writing() {
    let fixture = Fixture::new("lineage-identity-substitution");
    let stale = fixture.bundle("0.0.11");
    let target = fixture.bundle("0.0.12");
    fixture
        .adapter
        .execute(&fixture.install_plan(&target))
        .unwrap();
    fixture
        .adapter
        .replace_surface_for_test(DarwinHostSurface::Cache, &stale.snapshot)
        .unwrap();
    let plan = fixture
        .adapter
        .plan_repair_cache(&target.snapshot, &stale.snapshot, MARKETPLACE)
        .unwrap();
    fixture.adapter.execute(&plan).unwrap();

    let canonical = fs::read(fixture.lineage_record_path()).unwrap();
    let text = String::from_utf8(canonical.clone()).unwrap();
    let record: serde_json::Value = serde_json::from_slice(&canonical).unwrap();
    let valid_d = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    let valid_e = "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    let valid_f = "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    let reopened = DarwinHostTransactionAdapter::open(ConfinedRoot::open(&fixture.root).unwrap())
        .expect("canonical lineage and anchor reopen");
    assert_eq!(
        reopened.query(&plan).unwrap().diagnosis(),
        DarwinHostDiagnosis::Verified
    );
    assert!(fixture.lineage_anchor_record_path().is_file());

    let substitutions = [
        (
            "candidate-id",
            replace_after(
                &text,
                "\"target\":{",
                &format!(
                    "\"candidate_id\":\"{}\"",
                    record["target"]["candidate_id"].as_str().unwrap()
                ),
                &format!("\"candidate_id\":\"{FOREIGN_CANDIDATE}\""),
            ),
        ),
        (
            "plan-identity",
            replace_once(
                &text,
                &format!(
                    "\"plan_sha256\":\"{}\"",
                    record["plan_sha256"].as_str().unwrap()
                ),
                &format!("\"plan_sha256\":\"{valid_d}\""),
            ),
        ),
        (
            "package-identity",
            replace_after(
                &text,
                "\"target\":{",
                &format!(
                    "\"catalog_id\":\"{}\"",
                    record["target"]["catalog_id"].as_str().unwrap()
                ),
                &format!("\"catalog_id\":\"{valid_e}\""),
            ),
        ),
        (
            "operation",
            replace_once(
                &text,
                "\"operation\":\"repair-cache\"",
                "\"operation\":\"update\"",
            ),
        ),
        (
            "outcome",
            replace_once(
                &text,
                "\"outcome\":\"completed\"",
                "\"outcome\":\"cancelled\"",
            ),
        ),
        (
            "prior-lineage",
            replace_once(
                &text,
                &format!(
                    "\"previous_lineage_sha256\":\"{}\"",
                    record["previous_lineage_sha256"].as_str().unwrap()
                ),
                &format!("\"previous_lineage_sha256\":\"{valid_f}\""),
            ),
        ),
        (
            "generation",
            replace_once(&text, "\"generation\":2", "\"generation\":3"),
        ),
        (
            "terminal-digest",
            replace_after(
                &text,
                "\"terminal\":[",
                &format!(
                    "\"tree_sha256\":\"{}\"",
                    record["terminal"][0]["tree_sha256"].as_str().unwrap()
                ),
                &format!("\"tree_sha256\":\"{valid_d}\""),
            ),
        ),
    ];

    for (label, replacement) in substitutions {
        assert_ne!(replacement.as_bytes(), canonical, "{label} did not mutate");
        fs::write(fixture.lineage_record_path(), replacement).unwrap();
        let before = fixture.tree();
        let error =
            match DarwinHostTransactionAdapter::open(ConfinedRoot::open(&fixture.root).unwrap()) {
                Ok(adapter) => adapter.query(&plan).expect_err(label),
                Err(error) => error,
            };
        assert!(
            matches!(
                error.id(),
                DarwinHostErrorId::LineageCorrupt | DarwinHostErrorId::LineageConflict
            ),
            "{label} returned {error:?}"
        );
        assert_eq!(
            fixture.tree(),
            before,
            "{label} changed confined tree bytes"
        );
        fs::write(fixture.lineage_record_path(), &canonical).unwrap();
    }
}

#[test]
fn lineage_anchor_mode_drift_is_rejected_without_writing() {
    let fixture = installed_fixture("lineage-anchor-mode-drift");
    fs::set_permissions(
        fixture.lineage_anchor_record_path(),
        fs::Permissions::from_mode(0o666),
    )
    .unwrap();
    let before = fixture.tree();
    let error = match DarwinHostTransactionAdapter::open(ConfinedRoot::open(&fixture.root).unwrap())
    {
        Ok(_) => panic!("unsafe lineage anchor reopened"),
        Err(error) => error,
    };
    assert_eq!(error.id(), DarwinHostErrorId::UnsafeObject);
    assert_eq!(fixture.tree(), before);
}
