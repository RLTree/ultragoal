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
