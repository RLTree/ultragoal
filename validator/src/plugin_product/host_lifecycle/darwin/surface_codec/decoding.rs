use super::*;

pub(super) fn split_surface_bytes(
    expected_surface: DarwinHostSurface,
    bytes: &[u8],
) -> Result<(&[u8], Option<&[u8]>), DarwinHostError> {
    if bytes.len() > SURFACE_LIMIT {
        return Err(DarwinHostError::at(
            DarwinHostErrorId::ObjectTooLarge,
            expected_surface,
        ));
    }
    let (header, archive) = if matches!(
        expected_surface,
        DarwinHostSurface::Installed | DarwinHostSurface::Cache
    ) {
        if bytes.len() < ARCHIVE_MAGIC.len() + 4 || &bytes[..ARCHIVE_MAGIC.len()] != ARCHIVE_MAGIC {
            return Err(DarwinHostError::at(
                DarwinHostErrorId::SurfaceConflict,
                expected_surface,
            ));
        }
        let length_offset = ARCHIVE_MAGIC.len();
        let header_len = u32::from_be_bytes(
            bytes[length_offset..length_offset + 4]
                .try_into()
                .expect("fixed header length"),
        ) as usize;
        let header_start = length_offset + 4;
        let header_end = header_start.checked_add(header_len).ok_or_else(|| {
            DarwinHostError::at(DarwinHostErrorId::ObjectTooLarge, expected_surface)
        })?;
        if header_len > HEADER_LIMIT || header_end > bytes.len() {
            return Err(DarwinHostError::at(
                DarwinHostErrorId::SurfaceConflict,
                expected_surface,
            ));
        }
        (&bytes[header_start..header_end], Some(&bytes[header_end..]))
    } else {
        (bytes, None)
    };
    Ok((header, archive))
}

pub(super) fn validate_payload(
    record: &SurfaceRecord,
    archive: Option<&[u8]>,
    expected_surface: DarwinHostSurface,
) -> Result<(), DarwinHostError> {
    if record.schema != "harness-ultragoal.darwin-host-surface.v1" {
        return Err(DarwinHostError::at(
            DarwinHostErrorId::SurfaceConflict,
            expected_surface,
        ));
    }
    if record.surface != expected_surface {
        return Err(DarwinHostError::at(
            DarwinHostErrorId::CrossSurfaceSubstitution,
            expected_surface,
        ));
    }
    if record.marketplace != SUPPORTED_MARKETPLACE {
        return Err(DarwinHostError::at(
            DarwinHostErrorId::PackageSubstitution,
            expected_surface,
        ));
    }
    let valid = match (&record.payload, record.surface, archive) {
        (
            SurfacePayload::InstalledArchive {
                archive_byte_length,
            },
            DarwinHostSurface::Installed,
            Some(bytes),
        )
        | (
            SurfacePayload::CachedArchive {
                archive_byte_length,
            },
            DarwinHostSurface::Cache,
            Some(bytes),
        ) => {
            *archive_byte_length == bytes.len() as u64
                && digest(bytes) == record.package.archive_sha256
                && digest(bytes) == record.package.package_sha256
        }
        (
            SurfacePayload::MarketplaceCatalog { enabled: true },
            DarwinHostSurface::Marketplace,
            None,
        )
        | (
            SurfacePayload::AppRegistration { registered: true },
            DarwinHostSurface::AppRegistry,
            None,
        )
        | (
            SurfacePayload::PluginsUiVisibility { visible: true },
            DarwinHostSurface::PluginsUi,
            None,
        )
        | (
            SurfacePayload::DiscoveryIndex { discoverable: true },
            DarwinHostSurface::Discovery,
            None,
        )
        | (SurfacePayload::RuntimeIdentity { active: true }, DarwinHostSurface::Runtime, None) => {
            true
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(DarwinHostError::at(
            DarwinHostErrorId::SurfaceConflict,
            expected_surface,
        ))
    }
}
