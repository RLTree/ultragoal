use super::model::{PS_PLUGIN_MANIFEST, RouteSpec, spec};

pub(super) const ROUTES: [RouteSpec; 1] = [spec(
    "manifest-draft-to-ps-plugin-manifest",
    "LEGACY-MANIFEST-PROJECTION:plugin-manifest-draft.json",
    "legacy-manifest-projection-authority",
    "plugin-manifest-draft.json",
    "46bc8a5fe6362d85f2ef3890b18df8623625d30af233556acbae5a6dd1dd760f",
    &PS_PLUGIN_MANIFEST,
)];
