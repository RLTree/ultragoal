fn layout(bytes: &[u8]) -> Layout {
    let mut offset = 8;
    let metadata = (0..7).map(|_| string_range(bytes, &mut offset)).collect();
    let epoch = offset;
    offset += 8;
    let count = offset;
    let entry_count = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;
    let mut entries = Vec::new();
    for _ in 0..entry_count {
        let path = string_range(bytes, &mut offset);
        let mode = offset;
        offset += 4;
        let role = offset;
        offset += 1;
        let length = offset;
        let payload_length =
            u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap()) as usize;
        offset += 8;
        let digest = string_range(bytes, &mut offset);
        let payload = offset..offset + payload_length;
        offset += payload_length;
        entries.push(EntryLayout {
            path,
            mode,
            role,
            length,
            digest,
            payload,
        });
    }
    assert_eq!(offset, bytes.len());
    Layout {
        metadata,
        epoch,
        count,
        entries,
    }
}

fn string_range(bytes: &[u8], offset: &mut usize) -> Range<usize> {
    let length = u16::from_be_bytes(bytes[*offset..*offset + 2].try_into().unwrap()) as usize;
    *offset += 2;
    let range = *offset..*offset + length;
    *offset += length;
    range
}

fn candidate() -> (Fixture, PackagePlan, Vec<u8>) {
    let fixture = Fixture::new("candidate");
    let plan = fixture.plan();
    let archive = build_package(&plan, &mut Sink::default())
        .unwrap()
        .archive()
        .to_vec();
    (fixture, plan, archive)
}

fn assert_archive_error(plan: &PackagePlan, bytes: &[u8], expected: DistributionErrorId) {
    assert_eq!(verify_package(plan, bytes).unwrap_err().id(), expected);
}
