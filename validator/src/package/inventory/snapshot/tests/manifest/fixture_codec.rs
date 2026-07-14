use serde_json::json;
use std::fs;
use std::path::Path;

pub(super) struct ManifestFixtureRequest<'a> {
    pub(super) root: &'a Path,
    pub(super) resources: &'a [&'a str],
}

pub(super) struct ManifestFixtureResponse;

#[derive(Debug)]
pub(super) enum ManifestFixtureError {
    Encode,
    Write,
}

pub(super) fn write(
    request: ManifestFixtureRequest<'_>,
) -> Result<ManifestFixtureResponse, ManifestFixtureError> {
    let bytes = serde_json::to_vec(&json!({
        "name": "snapshot-test",
        "version": "0.0.0",
        "status": "test",
        "purpose": "test",
        "skills": [],
        "agents": [],
        "schemas": [],
        "fixtures": [],
        "authorable_templates": [],
        "generated_examples": [],
        "resources": request.resources,
        "schema_catalog": "schemas/catalog.json",
        "optional_connectors": [],
        "non_goals": []
    }))
    .map_err(|_| ManifestFixtureError::Encode)?;
    fs::write(request.root.join("plugin-manifest-draft.json"), bytes)
        .map_err(|_| ManifestFixtureError::Write)?;
    Ok(ManifestFixtureResponse)
}
