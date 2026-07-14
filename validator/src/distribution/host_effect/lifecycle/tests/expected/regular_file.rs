fn expected_regular(
    name: &str,
    byte_length: u64,
    mode: u32,
    content_sha256: String,
    data_synced: bool,
) -> ExpectedPublicationObjectIdentity {
    ExpectedPublicationObjectIdentity::regular(ExpectedRegularPublicationObject {
        name: name.to_owned(),
        byte_length,
        mode,
        hard_links: 1,
        content_sha256,
        data_synced,
    })
    .unwrap()
}

fn expected_missing(name: &str) -> ExpectedPublicationObjectIdentity {
    ExpectedPublicationObjectIdentity::missing(name.to_owned()).unwrap()
}

fn publication_expectation(prior: ExpectedPublicationObjectIdentity) -> PublicationExpectation {
    publication_expectation_with_effect(d('e'), prior)
}

fn publication_expectation_with_effect(
    effect_identity_sha256: String,
    prior: ExpectedPublicationObjectIdentity,
) -> PublicationExpectation {
    publication_expectation_exact(PublicationExpectationExact {
        effect_identity_sha256,
        prior,
        target_name: "ledger.json",
        temporary_name: ".ledger.json.0123456789abcdef.tmp",
        byte_length: 200,
        mode: 0o100400,
        content_sha256: d('2'),
        data_synced: true,
    })
    .unwrap()
}

struct PublicationExpectationExact<'a> {
    effect_identity_sha256: String,
    prior: ExpectedPublicationObjectIdentity,
    target_name: &'a str,
    temporary_name: &'a str,
    byte_length: u64,
    mode: u32,
    content_sha256: String,
    data_synced: bool,
}

fn publication_expectation_exact(
    request: PublicationExpectationExact<'_>,
) -> Result<PublicationExpectation, SupportedHostLifecycleError> {
    let PublicationExpectationExact {
        effect_identity_sha256,
        prior,
        target_name,
        temporary_name,
        byte_length,
        mode,
        content_sha256,
        data_synced,
    } = request;
    PublicationExpectation::new(
        effect_identity_sha256,
        prior,
        ExpectedPublicationObjectIdentity::regular(ExpectedRegularPublicationObject {
            name: target_name.to_owned(),
            byte_length,
            mode,
            hard_links: 1,
            content_sha256: content_sha256.clone(),
            data_synced,
        })?,
        ExpectedPublicationObjectIdentity::regular(ExpectedRegularPublicationObject {
            name: temporary_name.to_owned(),
            byte_length,
            mode,
            hard_links: 1,
            content_sha256,
            data_synced,
        })?,
    )
}

fn recovery_head(generation: u64, digest_byte: char) -> HostEffectLedgerHead {
    HostEffectLedgerHead::new(generation, d(digest_byte)).unwrap()
}

fn inventory(
    target: PublicationObjectObservation,
    temporary_objects: Vec<PublicationObjectObservation>,
    expectation: PublicationExpectation,
    current_ledger_head: HostEffectLedgerHead,
    acknowledgement: Option<PublicationAcknowledgementIdentity>,
) -> PublicationInventoryObservation {
    inventory_at(InventoryAt {
        scan_generation_before: 10,
        scan_generation_after: 10,
        target,
        temporary_objects,
        expectation,
        current_ledger_head,
        acknowledgement,
    })
}

struct InventoryAt {
    scan_generation_before: u64,
    scan_generation_after: u64,
    target: PublicationObjectObservation,
    temporary_objects: Vec<PublicationObjectObservation>,
    expectation: PublicationExpectation,
    current_ledger_head: HostEffectLedgerHead,
    acknowledgement: Option<PublicationAcknowledgementIdentity>,
}

fn inventory_at(request: InventoryAt) -> PublicationInventoryObservation {
    let InventoryAt {
        scan_generation_before,
        scan_generation_after,
        target,
        temporary_objects,
        expectation,
        current_ledger_head,
        acknowledgement,
    } = request;
    PublicationInventoryObservation::new(PublicationInventoryObservationRequest {
        scan_generation_before,
        scan_generation_after,
        target,
        temporary_objects,
        expectation,
        current_ledger_head,
        acknowledgement,
    })
    .unwrap()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SnapshotRow {
    relative: String,
    kind: &'static str,
    mode: u32,
    byte_length: u64,
    content_sha256: Option<String>,
}

fn recursive_snapshot(root: &Path) -> Vec<SnapshotRow> {
    let mut rows = Vec::new();
    snapshot_directory(root, root, &mut rows);
    rows.sort_by(|left, right| left.relative.cmp(&right.relative));
    rows
}

fn snapshot_directory(root: &Path, current: &Path, rows: &mut Vec<SnapshotRow>) {
    let mut entries = fs::read_dir(current)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).unwrap();
        let relative = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        #[cfg(unix)]
        let mode = metadata.mode();
        #[cfg(not(unix))]
        let mode = 0;
        if metadata.is_dir() {
            rows.push(SnapshotRow {
                relative,
                kind: "directory",
                mode,
                byte_length: 0,
                content_sha256: None,
            });
            snapshot_directory(root, &path, rows);
        } else if metadata.is_file() {
            let bytes = fs::read(&path).unwrap();
            rows.push(SnapshotRow {
                relative,
                kind: "file",
                mode,
                byte_length: bytes.len() as u64,
                content_sha256: Some(digest(&bytes)),
            });
        } else {
            rows.push(SnapshotRow {
                relative,
                kind: "special",
                mode,
                byte_length: metadata.len(),
                content_sha256: None,
            });
        }
    }
}
