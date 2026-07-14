pub(crate) fn inventory(archive: &DecodedArchive) -> Result<(Vec<u8>, String), DistributionError> {
    #[derive(Serialize)]
    struct Inventory<'a> {
        schema: &'static str,
        context_id: &'a str,
        candidate_id: &'a str,
        plugin_id: &'a str,
        version: &'a str,
        catalog_id: &'a str,
        accepted_inventory_sha256: &'a str,
        source_tree_sha256: &'a str,
        source_date_epoch: u64,
        entries: Vec<InventoryEntry<'a>>,
    }
    #[derive(Serialize)]
    struct InventoryEntry<'a> {
        path: &'a str,
        object_type: &'static str,
        mode: u32,
        sha256: &'a str,
        byte_length: u64,
        role: super::spec::PackageRole,
    }
    let entries = archive
        .entries
        .iter()
        .map(|entry| InventoryEntry {
            path: &entry.path,
            object_type: "regular-file",
            mode: entry.mode,
            sha256: &entry.sha256,
            byte_length: entry.bytes.len() as u64,
            role: entry.role,
        })
        .collect();
    let bytes = serde_json::to_vec(&Inventory {
        schema: "harness-ultragoal.package-inventory.v1",
        context_id: &archive.context_id,
        candidate_id: &archive.candidate_id,
        plugin_id: &archive.plugin_id,
        version: &archive.version,
        catalog_id: &archive.catalog_id,
        accepted_inventory_sha256: &archive.accepted_inventory_sha256,
        source_tree_sha256: &archive.source_tree_sha256,
        source_date_epoch: archive.source_date_epoch,
        entries,
    })
    .map_err(|_| error(DistributionErrorId::InvalidSpec))?;
    let digest = sha256(&bytes);
    Ok((bytes, digest))
}

fn push_string(bytes: &mut Vec<u8>, value: &str) -> Result<(), DistributionError> {
    let length = u16::try_from(value.len()).map_err(|_| error(DistributionErrorId::InvalidSpec))?;
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn decode_role(code: u8) -> Result<PackageRole, DistributionError> {
    match code {
        1 => Ok(PackageRole::Manifest),
        2 => Ok(PackageRole::Skill),
        3 => Ok(PackageRole::Agent),
        4 => Ok(PackageRole::Documentation),
        5 => Ok(PackageRole::Executable),
        6 => Ok(PackageRole::Data),
        _ => Err(error(DistributionErrorId::ArchiveMismatch)),
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], DistributionError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| error(DistributionErrorId::ObjectTooLarge))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| error(DistributionErrorId::ArchiveMismatch))?;
        self.offset = end;
        Ok(value)
    }

    fn string(&mut self, maximum: usize) -> Result<String, DistributionError> {
        let length = self.u16()? as usize;
        if length > maximum {
            return Err(error(DistributionErrorId::ObjectTooLarge));
        }
        std::str::from_utf8(self.take(length)?)
            .map(str::to_owned)
            .map_err(|_| error(DistributionErrorId::ArchiveMismatch))
    }

    fn u8(&mut self) -> Result<u8, DistributionError> {
        self.take(1).map(|bytes| bytes[0])
    }

    fn u16(&mut self) -> Result<u16, DistributionError> {
        self.take(2)
            .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Result<u32, DistributionError> {
        self.take(4)
            .map(|bytes| u32::from_be_bytes(bytes.try_into().expect("four bytes")))
    }

    fn u64(&mut self) -> Result<u64, DistributionError> {
        self.take(8)
            .map(|bytes| u64::from_be_bytes(bytes.try_into().expect("eight bytes")))
    }

    fn finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}
