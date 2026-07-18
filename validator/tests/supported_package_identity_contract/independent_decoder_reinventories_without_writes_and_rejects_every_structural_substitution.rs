#[test]
fn independent_decoder_reinventories_without_writes_and_rejects_every_structural_substitution() {
    let (fixture, plan, archive) = candidate();
    let before = snapshot_tree(&fixture.0);
    let verified = verify_package(&plan, &archive).unwrap();
    assert_eq!(verified.archive(), archive);
    assert_eq!(
        snapshot_tree(&fixture.0),
        before,
        "verification is zero-write"
    );
    let index = layout(&archive);

    let mut mutations = Vec::new();
    let mut wrong_magic = archive.clone();
    wrong_magic[0] ^= 1;
    mutations.push(wrong_magic);
    let mut non_utf8 = archive.clone();
    non_utf8[index.metadata[0].start] = 0xff;
    mutations.push(non_utf8);
    let mut wrong_context = archive.clone();
    wrong_context[index.metadata[0].start + 7] = b'b';
    mutations.push(wrong_context);
    let mut timestamp = archive.clone();
    timestamp[index.epoch..index.epoch + 8].copy_from_slice(&1u64.to_be_bytes());
    mutations.push(timestamp);
    let mut zero_count = archive.clone();
    zero_count[index.count..index.count + 4].copy_from_slice(&0u32.to_be_bytes());
    mutations.push(zero_count);
    let mut wrong_mode = archive.clone();
    wrong_mode[index.entries[0].mode..index.entries[0].mode + 4]
        .copy_from_slice(&0o777u32.to_be_bytes());
    mutations.push(wrong_mode);
    let mut unknown_role = archive.clone();
    unknown_role[index.entries[0].role] = 255;
    mutations.push(unknown_role);
    let mut wrong_digest = archive.clone();
    let digest_byte = &mut wrong_digest[index.entries[0].digest.start + 7];
    *digest_byte = if *digest_byte == b'a' { b'b' } else { b'a' };
    mutations.push(wrong_digest);
    let mut wrong_payload = archive.clone();
    wrong_payload[index.entries[0].payload.start] ^= 1;
    mutations.push(wrong_payload);
    let mut reordered = archive.clone();
    reordered[index.entries[2].path.clone()].fill(b'0');
    mutations.push(reordered);
    let mut trailing = archive.clone();
    trailing.push(0);
    mutations.push(trailing);
    let truncated = archive[..archive.len() - 1].to_vec();
    mutations.push(truncated);

    for bytes in mutations {
        assert_archive_error(&plan, &bytes, DistributionErrorId::ArchiveMismatch);
    }

    let mut escape = archive.clone();
    let path = &index.entries[1].path;
    let replacement = format!("../{}", "x".repeat(path.len() - 3));
    escape[path.clone()].copy_from_slice(replacement.as_bytes());
    assert_archive_error(&plan, &escape, DistributionErrorId::InvalidPath);
}

#[test]
fn declared_count_entry_size_and_total_archive_limits_fail_before_unbounded_work() {
    let (_fixture, plan, archive) = candidate();
    let index = layout(&archive);
    let mut count = archive.clone();
    count[index.count..index.count + 4].copy_from_slice(&4097u32.to_be_bytes());
    assert_archive_error(&plan, &count, DistributionErrorId::ObjectTooLarge);

    let mut entry = archive.clone();
    entry[index.entries[0].length..index.entries[0].length + 8]
        .copy_from_slice(&(4u64 * 1024 * 1024 + 1).to_be_bytes());
    assert_archive_error(&plan, &entry, DistributionErrorId::ObjectTooLarge);

    let oversized = vec![0; 65 * 1024 * 1024 + 1];
    assert_archive_error(&plan, &oversized, DistributionErrorId::ObjectTooLarge);
}

#[test]
fn corrective_worker_result_is_typed_lease_bound_and_exact_set_verified() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    if !root.join(R3_WORK_PACKAGE_PATH).exists() || !root.join(R3_RESULT_PATH).exists() {
        eprintln!(
            "SUPPORTED_PACKAGE_IDENTITY_R3_FIXTURE_ABSENT: historical root-owned fixture is not present in this refreshed N04 branch"
        );
        return;
    }
    assert_eq!(
        digest(&fs::read(root.join(R3_WORK_PACKAGE_PATH)).unwrap()),
        R3_WORK_PACKAGE_SHA256,
        "receipt is bound to the exact root-issued corrective package"
    );
    let bytes = fs::read(root.join(R3_RESULT_PATH)).unwrap();
    let result = WorkerResultV1::parse_json(&bytes).expect("authoritative WorkerResultV1 parser");
    if result
        .artifacts
        .iter()
        .any(|row| !root.join(&row.path).exists())
    {
        eprintln!(
            "SUPPORTED_PACKAGE_IDENTITY_R3_ARTIFACT_ABSENT: historical artifact set is not present in this refreshed N04 branch"
        );
        return;
    }
    let (package, lease, policy) = corrective_lease();
    package.validate().expect("typed corrective work package");
    policy.validate().expect("typed corrective scope policy");
    lease
        .validate(&policy)
        .expect("typed corrective lease and policy binding");
    let before = result
        .artifacts
        .iter()
        .map(|row| (row.path.clone(), fs::read(root.join(&row.path)).unwrap()))
        .collect::<Vec<_>>();
    let verified = ArtifactWorkspace::new(root)
        .unwrap()
        .verify(&result, &lease, &package)
        .expect("ArtifactWorkspace exact digest and lease verification");
    let result_id = result
        .result_id()
        .expect("canonical WorkerResult result_id");
    assert_eq!(verified.result_id(), result_id);
    assert_eq!(verified.artifact_count(), 18);
    assert!(r3_artifact_set_is_exact(&result));
    assert_eq!(result.context_id, R3_CONTEXT);
    assert_eq!(result.candidate_identity["context_id"], R3_CONTEXT);
    assert_eq!(result.candidate_identity["candidate_id"], R3_CANDIDATE);
    assert_eq!(
        result.candidate_identity["work_package_sha256"],
        R3_WORK_PACKAGE_SHA256
    );
    assert_eq!(
        result.candidate_identity["root_head"],
        "1750d586f6561d9d6ba64887cdafa6a36f23e41e"
    );
    assert_eq!(
        result.candidate_identity["root_tree"],
        "bf2d3e53c4b9410df41fb94ddf9b56ea4cf1e58a"
    );
    let after = result
        .artifacts
        .iter()
        .map(|row| (row.path.clone(), fs::read(root.join(&row.path)).unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(after, before, "typed receipt verification is zero-write");

    let mut missing: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    missing["artifacts"].as_array_mut().unwrap().remove(0);
    let missing = WorkerResultV1::parse_json(&serde_json::to_vec(&missing).unwrap())
        .expect("generic parser accepts a structurally valid but incomplete listed set");
    assert!(
        !r3_artifact_set_is_exact(&missing),
        "root-sealed exact-set validation rejects a missing artifact"
    );

    let mut replaced: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    replaced["artifacts"].as_array_mut().unwrap()[0]["path"] =
        json!("validator/src/distribution/mod.rs");
    let replaced = WorkerResultV1::parse_json(&serde_json::to_vec(&replaced).unwrap())
        .expect("generic parser accepts a structurally valid substituted row");
    assert!(
        !r3_artifact_set_is_exact(&replaced),
        "root-sealed exact-set validation rejects an unknown replacement"
    );
    eprintln!("SUPPORTED_PACKAGE_IDENTITY_R3_RESULT_ID={result_id}");
}

fn r3_artifact_set_is_exact(result: &WorkerResultV1) -> bool {
    let expected = BTreeSet::from([
        "validator/src/distribution/cache.rs",
        "validator/src/distribution/host_capability.rs",
        "validator/src/distribution/host_effect/executor/tests.rs",
        "validator/src/distribution/host_effect/lifecycle/binding.rs",
        "validator/src/distribution/host_effect/lifecycle/tests.rs",
        "validator/src/distribution/identity.rs",
        "validator/src/distribution/marketplace.rs",
        "validator/src/distribution/package/archive.rs",
        "validator/src/distribution/package/snapshot.rs",
        "validator/src/distribution/registry_observation.rs",
        "validator/src/distribution/runtime_probe.rs",
        "validator/src/plugin_product/host_lifecycle/session.rs",
        "validator/tests/distribution_contract/isolated_journey.rs",
        "validator/tests/distribution_contract/journey_adversarial/mod.rs",
        "validator/tests/distribution_contract/observation_races.rs",
        "validator/tests/distribution_contract/runtime_session/mod.rs",
        "validator/tests/plugin_host_lifecycle_contract/negative.rs",
        "validator/tests/supported_package_identity_contract.rs",
    ]);
    result
        .artifacts
        .iter()
        .map(|row| row.path.as_str())
        .collect::<BTreeSet<_>>()
        == expected
}
