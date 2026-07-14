#[test]
fn expected_publication_identity_binds_every_field_and_rejects_unsafe_shapes() {
    let baseline =
        publication_expectation(expected_regular("ledger.json", 100, 0o100400, d('1'), true));
    let baseline_identity = baseline.publication_identity_sha256().to_owned();
    let variants = [
        publication_expectation_with_effect(
            d('f'),
            expected_regular("ledger.json", 100, 0o100400, d('1'), true),
        ),
        publication_expectation(expected_missing("ledger.json")),
        publication_expectation(expected_regular("ledger.json", 101, 0o100400, d('1'), true)),
        publication_expectation(expected_regular("ledger.json", 100, 0o100440, d('1'), true)),
        publication_expectation(expected_regular("ledger.json", 100, 0o100400, d('3'), true)),
        publication_expectation_exact(PublicationExpectationExact {
            effect_identity_sha256: d('e'),
            prior: expected_regular("ledger.json", 100, 0o100400, d('1'), true),
            target_name: "ledger.json",
            temporary_name: ".ledger.json.fedcba9876543210.tmp",
            byte_length: 200,
            mode: 0o100400,
            content_sha256: d('2'),
            data_synced: true,
        })
        .unwrap(),
        publication_expectation_exact(PublicationExpectationExact {
            effect_identity_sha256: d('e'),
            prior: expected_regular("ledger.json", 100, 0o100400, d('1'), true),
            target_name: "ledger.json",
            temporary_name: ".ledger.json.0123456789abcdef.tmp",
            byte_length: 201,
            mode: 0o100400,
            content_sha256: d('2'),
            data_synced: true,
        })
        .unwrap(),
        publication_expectation_exact(PublicationExpectationExact {
            effect_identity_sha256: d('e'),
            prior: expected_regular("ledger.json", 100, 0o100400, d('1'), true),
            target_name: "ledger.json",
            temporary_name: ".ledger.json.0123456789abcdef.tmp",
            byte_length: 200,
            mode: 0o100440,
            content_sha256: d('2'),
            data_synced: true,
        })
        .unwrap(),
        publication_expectation_exact(PublicationExpectationExact {
            effect_identity_sha256: d('e'),
            prior: expected_regular("ledger.json", 100, 0o100400, d('1'), true),
            target_name: "ledger.json",
            temporary_name: ".ledger.json.0123456789abcdef.tmp",
            byte_length: 200,
            mode: 0o100400,
            content_sha256: d('4'),
            data_synced: true,
        })
        .unwrap(),
        publication_expectation_exact(PublicationExpectationExact {
            effect_identity_sha256: d('e'),
            prior: expected_regular("state.json", 100, 0o100400, d('1'), true),
            target_name: "state.json",
            temporary_name: ".state.json.0123456789abcdef.tmp",
            byte_length: 200,
            mode: 0o100400,
            content_sha256: d('2'),
            data_synced: true,
        })
        .unwrap(),
    ];
    for changed in variants {
        assert_ne!(
            changed.publication_identity_sha256(),
            baseline_identity.as_str()
        );
    }

    for invalid in [
        ExpectedPublicationObjectIdentity::regular(ExpectedRegularPublicationObject {
            name: "ledger.json".to_owned(),
            byte_length: 100,
            mode: 0o100400,
            hard_links: 1,
            content_sha256: "not-a-digest".to_owned(),
            data_synced: true,
        }),
        ExpectedPublicationObjectIdentity::regular(ExpectedRegularPublicationObject {
            name: "ledger.json".to_owned(),
            byte_length: 100,
            mode: 0o040400,
            hard_links: 1,
            content_sha256: d('1'),
            data_synced: true,
        }),
        ExpectedPublicationObjectIdentity::regular(ExpectedRegularPublicationObject {
            name: "ledger.json".to_owned(),
            byte_length: 100,
            mode: 0o100400,
            hard_links: 2,
            content_sha256: d('1'),
            data_synced: true,
        }),
        ExpectedPublicationObjectIdentity::missing("../ledger.json".to_owned()),
    ] {
        assert_eq!(
            invalid.unwrap_err().id(),
            SupportedHostLifecycleErrorId::RecoveryUnsafe
        );
    }

    let exact_next = expected_regular("ledger.json", 200, 0o100400, d('2'), true);
    let exact_temp = expected_regular(
        ".ledger.json.0123456789abcdef.tmp",
        200,
        0o100400,
        d('2'),
        true,
    );
    let invalid_expectations = [
        PublicationExpectation::new(
            "not-a-digest".to_owned(),
            expected_missing("ledger.json"),
            exact_next.clone(),
            exact_temp.clone(),
        ),
        PublicationExpectation::new(
            d('e'),
            expected_regular("ledger.json", 100, 0o100600, d('1'), true),
            exact_next.clone(),
            exact_temp.clone(),
        ),
        PublicationExpectation::new(
            d('e'),
            expected_regular("ledger.json", 100, 0o100400, d('1'), false),
            exact_next.clone(),
            exact_temp.clone(),
        ),
        PublicationExpectation::new(
            d('e'),
            expected_missing("other.json"),
            exact_next.clone(),
            exact_temp.clone(),
        ),
        PublicationExpectation::new(
            d('e'),
            expected_missing("ledger.json"),
            expected_regular("ledger.json", 200, 0o100600, d('2'), true),
            expected_regular(
                ".ledger.json.0123456789abcdef.tmp",
                200,
                0o100600,
                d('2'),
                true,
            ),
        ),
        PublicationExpectation::new(
            d('e'),
            expected_missing("ledger.json"),
            expected_regular("ledger.json", 200, 0o100400, d('2'), false),
            expected_regular(
                ".ledger.json.0123456789abcdef.tmp",
                200,
                0o100400,
                d('2'),
                false,
            ),
        ),
        PublicationExpectation::new(
            d('e'),
            expected_missing("ledger.json"),
            exact_next.clone(),
            expected_regular(
                ".ledger.json.0123456789abcdef.tmp",
                201,
                0o100400,
                d('2'),
                true,
            ),
        ),
        PublicationExpectation::new(
            d('e'),
            expected_missing("ledger.json"),
            exact_next,
            expected_regular(
                ".ledger.json.not-canonical.tmp",
                200,
                0o100400,
                d('2'),
                true,
            ),
        ),
    ];
    for invalid in invalid_expectations {
        assert_eq!(
            invalid.unwrap_err().id(),
            SupportedHostLifecycleErrorId::RecoveryUnsafe
        );
    }
}
