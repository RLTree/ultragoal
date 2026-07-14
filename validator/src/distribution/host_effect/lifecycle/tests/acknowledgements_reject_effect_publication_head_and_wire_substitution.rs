#[test]
fn acknowledgements_reject_effect_publication_head_and_wire_substitution() {
    let expectation =
        publication_expectation(expected_regular("ledger.json", 100, 0o100400, d('1'), true));
    let target_next = regular_with_mode("ledger.json", 10, 200, 0o100400, Some(d('2')), true);
    let head = recovery_head(7, 'a');
    let acknowledgement = PublicationAcknowledgementIdentity::new(&expectation, &head).unwrap();
    let canonical = serde_json::to_vec(&acknowledgement).unwrap();
    let decoded = PublicationAcknowledgementIdentity::from_canonical_json(&canonical).unwrap();
    assert_eq!(
        inventory(
            target_next.clone(),
            vec![],
            expectation.clone(),
            head.clone(),
            Some(decoded),
        )
        .classify()
        .unwrap()
        .id(),
        PublicationClassificationId::AcknowledgedCommitted
    );

    let foreign_effect = publication_expectation_with_effect(
        d('f'),
        expected_regular("ledger.json", 100, 0o100400, d('1'), true),
    );
    let foreign_publication =
        publication_expectation(expected_regular("ledger.json", 99, 0o100400, d('3'), true));
    let substitutions = [
        PublicationAcknowledgementIdentity::new(&foreign_effect, &head).unwrap(),
        PublicationAcknowledgementIdentity::new(&foreign_publication, &head).unwrap(),
        PublicationAcknowledgementIdentity::new(&expectation, &recovery_head(6, 'a')).unwrap(),
        PublicationAcknowledgementIdentity::new(&expectation, &recovery_head(7, 'b')).unwrap(),
    ];
    for changed in substitutions {
        assert_eq!(
            inventory(
                target_next.clone(),
                vec![],
                expectation.clone(),
                head.clone(),
                Some(changed),
            )
            .classify()
            .unwrap()
            .id(),
            PublicationClassificationId::FalsePassReceipt
        );
    }

    let mut unknown = serde_json::to_value(&acknowledgement).unwrap();
    unknown
        .as_object_mut()
        .unwrap()
        .insert("unknown".to_owned(), serde_json::Value::Bool(true));
    let mut missing = serde_json::to_value(&acknowledgement).unwrap();
    missing
        .as_object_mut()
        .unwrap()
        .remove("publication_identity_sha256");
    let mut extra_head = serde_json::to_value(&acknowledgement).unwrap();
    extra_head["ledger_head"]
        .as_object_mut()
        .unwrap()
        .insert("extra".to_owned(), serde_json::Value::Bool(true));
    let mut noncanonical = serde_json::to_value(&acknowledgement).unwrap();
    noncanonical["acknowledgement_sha256"] = serde_json::Value::String(d('0'));
    for malformed in [unknown, missing, extra_head, noncanonical] {
        assert_eq!(
            PublicationAcknowledgementIdentity::from_canonical_json(
                &serde_json::to_vec(&malformed).unwrap()
            )
            .unwrap_err()
            .id(),
            SupportedHostLifecycleErrorId::RecoveryUnsafe
        );
    }
    let canonical_value = serde_json::from_slice::<serde_json::Value>(&canonical).unwrap();
    let canonical_object = canonical_value.as_object().unwrap();
    let mut reordered_object = serde_json::Map::new();
    for key in [
        "acknowledgement_sha256",
        "ledger_head",
        "publication_identity_sha256",
        "effect_identity_sha256",
        "schema_version",
    ] {
        reordered_object.insert(key.to_owned(), canonical_object[key].clone());
    }
    let reordered = serde_json::to_vec(&serde_json::Value::Object(reordered_object)).unwrap();
    assert_ne!(reordered, canonical);
    let mut whitespace = b" ".to_vec();
    whitespace.extend_from_slice(&canonical);
    whitespace.push(b'\n');
    for noncanonical_encoding in [reordered, whitespace] {
        assert_eq!(
            PublicationAcknowledgementIdentity::from_canonical_json(&noncanonical_encoding)
                .unwrap_err()
                .id(),
            SupportedHostLifecycleErrorId::RecoveryUnsafe
        );
    }
    let duplicate = String::from_utf8(canonical).unwrap().replacen(
        '{',
        "{\"schema_version\":\"PublicationAcknowledgementIdentity-v1\",",
        1,
    );
    assert_eq!(
        PublicationAcknowledgementIdentity::from_canonical_json(duplicate.as_bytes())
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::RecoveryUnsafe
    );
}
