#[test]
fn research_source_record_fields_and_canonical_binding_fail_closed() {
    let record = research_record(
        "source-primary",
        ResearchSourceClass::PrimarySpecification,
        FactTemporalScope::MutableCapability,
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let bytes = record.canonical_bytes();
    assert!(
        ResearchSource::from_bound_record(
            BoundInput::regular(
                "research/source-primary.json",
                digest(&bytes),
                bytes.len() as u64
            ),
            bytes.clone(),
        )
        .is_ok()
    );

    for required_field in [
        "schema_version",
        "source_id",
        "publisher",
        "url",
        "source_class",
        "checked_date",
        "observed_at_epoch_seconds",
        "valid_until_epoch_seconds",
        "verified_source_facts",
        "binding_product_requirements",
        "advisory_practices",
        "experimental_hypotheses",
        "rejected_recommendations",
        "limitations",
        "mapped_law_ids",
        "supports_proposal_ids",
    ] {
        let mut value = serde_json::from_slice::<Value>(&bytes).unwrap();
        value.as_object_mut().unwrap().remove(required_field);
        let missing = serde_json::to_vec(&value).unwrap();
        assert!(
            ResearchSource::from_bound_record(
                BoundInput::regular(
                    "research/source-primary.json",
                    digest(&missing),
                    missing.len() as u64,
                ),
                missing,
            )
            .is_err(),
            "missing source field passed: {required_field}"
        );
    }
    for required_field in [
        "schema_version",
        "source_id",
        "publisher",
        "url",
        "source_class",
        "checked_date",
        "observed_at_epoch_seconds",
        "valid_until_epoch_seconds",
        "verified_source_facts",
        "binding_product_requirements",
        "advisory_practices",
        "experimental_hypotheses",
        "rejected_recommendations",
        "limitations",
        "mapped_law_ids",
        "supports_proposal_ids",
    ] {
        let mut value = serde_json::from_slice::<Value>(&bytes).unwrap();
        value[required_field] = match required_field {
            "observed_at_epoch_seconds" | "valid_until_epoch_seconds" => json!(0),
            "verified_source_facts"
            | "binding_product_requirements"
            | "advisory_practices"
            | "experimental_hypotheses"
            | "rejected_recommendations"
            | "limitations"
            | "mapped_law_ids"
            | "supports_proposal_ids" => json!([]),
            _ => json!(""),
        };
        let empty = serde_json::to_vec(&value).unwrap();
        assert!(
            ResearchSource::from_bound_record(
                BoundInput::regular(
                    "research/source-primary.json",
                    digest(&empty),
                    empty.len() as u64,
                ),
                empty,
            )
            .is_err(),
            "empty source field passed: {required_field}"
        );
    }
    let mut unknown_value = serde_json::from_slice::<Value>(&bytes).unwrap();
    unknown_value["unknown_authority"] = json!("forbidden");
    let unknown = serde_json::to_vec(&unknown_value).unwrap();
    assert!(
        ResearchSource::from_bound_record(
            BoundInput::regular(
                "research/source-primary.json",
                digest(&unknown),
                unknown.len() as u64,
            ),
            unknown,
        )
        .is_err(),
        "unknown source field passed"
    );

    let pretty = serde_json::to_vec_pretty(&record).unwrap();
    assert!(
        ResearchSource::from_bound_record(
            BoundInput::regular(
                "research/source-primary.json",
                digest(&pretty),
                pretty.len() as u64,
            ),
            pretty,
        )
        .is_err(),
        "noncanonical source bytes passed"
    );
    assert!(
        ResearchSource::from_bound_record(
            BoundInput::regular("research/source-primary.json", sha('f'), bytes.len() as u64),
            bytes.clone(),
        )
        .is_err(),
        "digest substitution passed"
    );
    assert!(
        ResearchSource::from_bound_record(
            BoundInput::regular(
                "research/source-primary.json",
                digest(&bytes),
                bytes.len() as u64 + 1,
            ),
            bytes.clone(),
        )
        .is_err(),
        "length substitution passed"
    );
    for snapshot in [
        BoundInput::regular("../source.json", digest(&bytes), bytes.len() as u64),
        BoundInput::observed(
            "research/source-primary.json",
            digest(&bytes),
            bytes.len() as u64,
            1,
            InputKind::Symlink,
        ),
        BoundInput::observed(
            "research/source-primary.json",
            digest(&bytes),
            bytes.len() as u64,
            2,
            InputKind::Regular,
        ),
    ] {
        assert!(
            ResearchSource::from_bound_record(snapshot, bytes.clone()).is_err(),
            "unsafe source snapshot passed"
        );
    }
    assert!(
        ResearchSource::from_bound_record(
            BoundInput::regular("research/source-primary.json", digest(&[]), 0),
            Vec::new(),
        )
        .is_err(),
        "empty source bytes passed"
    );

    let substituted_record = research_record(
        "source-substituted",
        ResearchSourceClass::PrimarySpecification,
        FactTemporalScope::MutableCapability,
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let forged = ResearchSource::test_only_unchecked(
        BoundInput::regular(
            "research/source-primary.json",
            digest(&bytes),
            bytes.len() as u64,
        ),
        substituted_record,
        bytes,
    );
    research_source_rejected(forged, "research-source-invalid");
}
