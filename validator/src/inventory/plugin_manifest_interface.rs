use super::plugin_manifest::Problem;
use crate::context::ReadSession;
use crate::plugin_manifest::PluginInterface;
use std::path::Path;

pub(super) fn validate_assets(
    reads: &ReadSession,
    root: &Path,
    interface: &PluginInterface,
    problems: &mut Vec<Problem>,
) {
    for asset in [
        &interface.composer_icon,
        &interface.logo,
        &interface.logo_dark,
    ]
    .into_iter()
    .flatten()
    {
        if !super::plugin_manifest_path::asset_exists(reads, root, asset, false) {
            problems.push((
                "invalid_plugin_asset",
                "source plugin manifest asset is absent or unsafe",
            ));
        }
    }
    if interface
        .screenshots
        .iter()
        .any(|asset| !super::plugin_manifest_path::asset_exists(reads, root, asset, true))
    {
        problems.push((
            "invalid_plugin_screenshot",
            "source plugin manifest screenshot is absent or unsafe",
        ));
    }
}
