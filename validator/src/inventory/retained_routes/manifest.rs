use super::model::{PS_PLUGIN_MANIFEST, RouteSpec, spec};

pub(super) const ROUTES: [RouteSpec; 1] = [spec(
    "manifest-draft-to-ps-plugin-manifest",
    "LEGACY-MANIFEST-PROJECTION:plugin-manifest-draft.json",
    "legacy-manifest-projection-authority",
    "plugin-manifest-draft.json",
    "f40c0bf21d87a72fc819af278501a0822a0e1c06f211794bcafbd0ffc8596d17",
    &PS_PLUGIN_MANIFEST,
)];
