use super::contracts::{RedCatalogError, RedCatalogRow};

pub(crate) struct RedCatalogRenderRequest<'a> {
    pub(crate) rows: &'a [RedCatalogRow],
}

pub(crate) struct RedCatalogRenderResponse {
    pub(crate) bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RedCatalogRenderError {
    SerializationFailed,
}

pub(crate) fn emit(
    request: RedCatalogRenderRequest<'_>,
) -> Result<RedCatalogRenderResponse, RedCatalogRenderError> {
    let mut bytes = serde_json::to_vec_pretty(request.rows)
        .map_err(|_| RedCatalogRenderError::SerializationFailed)?;
    bytes.push(b'\n');
    Ok(RedCatalogRenderResponse { bytes })
}

pub(super) fn render(rows: &[RedCatalogRow]) -> Result<Vec<u8>, RedCatalogError> {
    emit(RedCatalogRenderRequest { rows })
        .map(|response| response.bytes)
        .map_err(|_| RedCatalogError::new("red_catalog_render_failed", None))
}
