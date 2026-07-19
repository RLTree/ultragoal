use super::model::{PS_PLUGIN_MANIFEST, RouteSpec, spec};

pub(super) const ROUTES: [RouteSpec; 1] = [spec(
    "manifest-draft-to-ps-plugin-manifest",
    "LEGACY-MANIFEST-PROJECTION:plugin-manifest-draft.json",
    "legacy-manifest-projection-authority",
    "plugin-manifest-draft.json",
    "5c5c4e096366a396a677b193b48376b109e90fca496a610ee6690c6a58dc9420",
    &PS_PLUGIN_MANIFEST,
)];
