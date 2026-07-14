use super::{DarwinHostError, DarwinHostErrorId, DarwinHostSurface, SUPPORTED_MARKETPLACE};
use crate::distribution::{TreeObject, tree_sha256};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const ARCHIVE_MAGIC: &[u8; 8] = b"HULHOST1";
const HEADER_LIMIT: usize = 64 * 1024;
pub(super) const SURFACE_LIMIT: usize = 65 * 1024 * 1024;

mod package_binding;
pub(super) use package_binding::PackageBinding;

#[derive(Clone, Copy)]
pub(super) enum DarwinSurfaceCodecRequest<'a> {
    Encode {
        surface: DarwinHostSurface,
        root_id: &'a str,
        package: &'a PackageBinding,
        archive: &'a [u8],
    },
    Decode {
        expected_surface: DarwinHostSurface,
        bytes: &'a [u8],
    },
    Digest(&'a [u8]),
}

pub(super) enum DarwinSurfaceCodecResponse {
    Bytes(Vec<u8>),
    Record(SurfaceRecord),
    Digest(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DarwinSurfaceCodecError {
    Encode,
    Decode,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum SurfacePayload {
    InstalledArchive { archive_byte_length: u64 },
    CachedArchive { archive_byte_length: u64 },
    MarketplaceCatalog { enabled: bool },
    AppRegistration { registered: bool },
    PluginsUiVisibility { visible: bool },
    DiscoveryIndex { discoverable: bool },
    RuntimeIdentity { active: bool },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SurfaceRecord {
    schema: String,
    pub(super) surface: DarwinHostSurface,
    pub(super) root_id: String,
    pub(super) marketplace: String,
    pub(super) package: PackageBinding,
    payload: SurfacePayload,
}

fn encode_impl(
    surface: DarwinHostSurface,
    root_id: &str,
    package: &PackageBinding,
    archive: &[u8],
) -> Result<Vec<u8>, DarwinHostError> {
    let payload = match surface {
        DarwinHostSurface::Installed => SurfacePayload::InstalledArchive {
            archive_byte_length: archive.len() as u64,
        },
        DarwinHostSurface::Cache => SurfacePayload::CachedArchive {
            archive_byte_length: archive.len() as u64,
        },
        DarwinHostSurface::Marketplace => SurfacePayload::MarketplaceCatalog { enabled: true },
        DarwinHostSurface::AppRegistry => SurfacePayload::AppRegistration { registered: true },
        DarwinHostSurface::PluginsUi => SurfacePayload::PluginsUiVisibility { visible: true },
        DarwinHostSurface::Discovery => SurfacePayload::DiscoveryIndex { discoverable: true },
        DarwinHostSurface::Runtime => SurfacePayload::RuntimeIdentity { active: true },
    };
    let record = SurfaceRecord {
        schema: "harness-ultragoal.darwin-host-surface.v1".into(),
        surface,
        root_id: root_id.to_owned(),
        marketplace: SUPPORTED_MARKETPLACE.into(),
        package: package.clone(),
        payload,
    };
    let header = serde_json::to_vec(&record)
        .map_err(|_| DarwinHostError::at(DarwinHostErrorId::InvalidPackage, surface))?;
    if header.len() > HEADER_LIMIT {
        return Err(DarwinHostError::at(
            DarwinHostErrorId::ObjectTooLarge,
            surface,
        ));
    }
    if matches!(
        surface,
        DarwinHostSurface::Installed | DarwinHostSurface::Cache
    ) {
        let total = ARCHIVE_MAGIC.len() + 4 + header.len() + archive.len();
        if total > SURFACE_LIMIT {
            return Err(DarwinHostError::at(
                DarwinHostErrorId::ObjectTooLarge,
                surface,
            ));
        }
        let mut bytes = Vec::with_capacity(total);
        bytes.extend_from_slice(ARCHIVE_MAGIC);
        bytes.extend_from_slice(&(header.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&header);
        bytes.extend_from_slice(archive);
        Ok(bytes)
    } else {
        Ok(header)
    }
}

mod decoding;
use decoding::{split_surface_bytes, validate_payload};

fn decode_impl(
    expected_surface: DarwinHostSurface,
    bytes: &[u8],
) -> Result<SurfaceRecord, DarwinHostError> {
    let (header, archive) = split_surface_bytes(expected_surface, bytes)?;
    let record: SurfaceRecord = serde_json::from_slice(header)
        .map_err(|_| DarwinHostError::at(DarwinHostErrorId::SurfaceConflict, expected_surface))?;
    if serde_json::to_vec(&record).ok().as_deref() != Some(header) {
        return Err(DarwinHostError::at(
            DarwinHostErrorId::SurfaceConflict,
            expected_surface,
        ));
    }
    validate_payload(&record, archive, expected_surface)?;
    Ok(record)
}

fn digest_impl(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(super) fn execute(
    request: DarwinSurfaceCodecRequest<'_>,
) -> Result<DarwinSurfaceCodecResponse, DarwinSurfaceCodecError> {
    match request {
        DarwinSurfaceCodecRequest::Encode {
            surface,
            root_id,
            package,
            archive,
        } => encode_impl(surface, root_id, package, archive)
            .map(DarwinSurfaceCodecResponse::Bytes)
            .map_err(|_| DarwinSurfaceCodecError::Encode),
        DarwinSurfaceCodecRequest::Decode {
            expected_surface,
            bytes,
        } => decode_impl(expected_surface, bytes)
            .map(DarwinSurfaceCodecResponse::Record)
            .map_err(|_| DarwinSurfaceCodecError::Decode),
        DarwinSurfaceCodecRequest::Digest(bytes) => {
            Ok(DarwinSurfaceCodecResponse::Digest(digest_impl(bytes)))
        }
    }
}

pub(super) fn encode_surface(
    surface: DarwinHostSurface,
    root_id: &str,
    package: &PackageBinding,
    archive: &[u8],
) -> Result<Vec<u8>, DarwinHostError> {
    match execute(DarwinSurfaceCodecRequest::Encode {
        surface,
        root_id,
        package,
        archive,
    }) {
        Ok(DarwinSurfaceCodecResponse::Bytes(bytes)) => Ok(bytes),
        _ => Err(DarwinHostError::at(
            DarwinHostErrorId::InvalidPackage,
            surface,
        )),
    }
}

pub(in super::super) fn decode_surface(
    expected_surface: DarwinHostSurface,
    bytes: &[u8],
) -> Result<SurfaceRecord, DarwinHostError> {
    match execute(DarwinSurfaceCodecRequest::Decode {
        expected_surface,
        bytes,
    }) {
        Ok(DarwinSurfaceCodecResponse::Record(record)) => Ok(record),
        _ => Err(DarwinHostError::at(
            DarwinHostErrorId::SurfaceConflict,
            expected_surface,
        )),
    }
}

pub(super) fn surface_tree(bytes: Vec<u8>) -> Vec<TreeObject> {
    vec![TreeObject::regular("record".into(), 0o644, bytes)]
}

pub(super) fn surface_tree_sha256(bytes: &[u8]) -> Result<String, DarwinHostError> {
    tree_sha256(&surface_tree(bytes.to_vec()))
        .map_err(|_| DarwinHostError::new(DarwinHostErrorId::InvalidPackage))
}

pub(super) fn digest(bytes: &[u8]) -> String {
    match execute(DarwinSurfaceCodecRequest::Digest(bytes)) {
        Ok(DarwinSurfaceCodecResponse::Digest(value)) => value,
        _ => unreachable!("digest response is total"),
    }
}
