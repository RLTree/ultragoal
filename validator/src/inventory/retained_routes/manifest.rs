use super::model::{PS_PLUGIN_MANIFEST, RouteSpec, spec};

pub(super) const ROUTES: [RouteSpec; 1] = [spec(
    "manifest-draft-to-ps-plugin-manifest",
    "LEGACY-MANIFEST-PROJECTION:plugin-manifest-draft.json",
    "legacy-manifest-projection-authority",
    "plugin-manifest-draft.json",
    "7351910b9bb9b5bb45dd14b20c53730dfeebffc1ee4162ae2d8ec686dc51b762",
    &PS_PLUGIN_MANIFEST,
)];
