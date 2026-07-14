use super::contracts::{
    RedCatalogError, RedCatalogProjection, RedCatalogProjectionRequest, RedCatalogRow,
};
use super::{emitter, filesystem, packet_parser};
use std::collections::BTreeSet;

pub(crate) fn render(
    request: RedCatalogProjectionRequest<'_>,
) -> Result<RedCatalogProjection, RedCatalogError> {
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut rows = Vec::new();
    for source in filesystem::packet_sources(request.root)? {
        let packet = packet_parser::packet(&source.bytes, &source.relative)?;
        let expected_path = format!("fixtures/red/{}.json", packet.id);
        if !ids.insert(packet.id.clone()) {
            return Err(RedCatalogError::new(
                "red_catalog_packet_id_duplicate",
                Some(packet.id),
            ));
        }
        if source.relative != expected_path {
            return Err(RedCatalogError::new(
                "red_catalog_packet_identity_path_mismatch",
                Some(source.relative),
            ));
        }
        if !paths.insert(source.relative.clone()) {
            return Err(RedCatalogError::new(
                "red_catalog_packet_path_duplicate",
                Some(source.relative),
            ));
        }
        rows.push(RedCatalogRow {
            expected_failure: packet.expected_failure,
            id: packet.id,
            packet_digest: crate::digest::bytes(&source.bytes),
            packet_path: source.relative,
        });
    }
    if rows.is_empty() {
        return Err(RedCatalogError::new("red_catalog_packet_set_empty", None));
    }
    rows.sort_by(|left, right| left.id.cmp(&right.id));
    let bytes = emitter::render(&rows)?;
    Ok(RedCatalogProjection { bytes, rows })
}

pub(crate) fn check(
    request: RedCatalogProjectionRequest<'_>,
) -> Result<RedCatalogProjection, RedCatalogError> {
    let root = request.root;
    let projection = render(RedCatalogProjectionRequest { root })?;
    let current = filesystem::catalog_bytes(root)?;
    if current != projection.bytes {
        return Err(RedCatalogError::new("red_catalog_projection_drift", None));
    }
    Ok(projection)
}
