use super::model::{PS_PLUGIN_MANIFEST, RouteSpec, spec};

pub(super) const ROUTES: [RouteSpec; 1] = [spec(
    "manifest-draft-to-ps-plugin-manifest",
    "LEGACY-MANIFEST-PROJECTION:plugin-manifest-draft.json",
    "legacy-manifest-projection-authority",
    "plugin-manifest-draft.json",
    "6df14610435f3e565e57952655d68bb06ad165130b8a57697943aeb95ef8d672",
    &PS_PLUGIN_MANIFEST,
)];
