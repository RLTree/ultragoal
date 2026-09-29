use std::collections::BTreeSet;
use std::path::Path;

mod discovered;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthoritySurfaceInventoryRow {
    pub(crate) role: String,
    pub(crate) path: String,
    pub(crate) package_inventory_required: bool,
    pub(crate) existence_required: bool,
    pub(crate) exists_on_disk: bool,
    pub(crate) listed_in_package_inventory: bool,
}

pub(super) fn rows(root: &Path, inventory: &BTreeSet<String>) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut rows = super::inventory_requirements::required_surfaces()
        .iter()
        .map(|surface| AuthoritySurfaceInventoryRow {
            role: surface.role.to_string(),
            path: surface.rel.to_string(),
            package_inventory_required: surface.package_inventory_required,
            existence_required: surface.existence_required,
            exists_on_disk: root.join(surface.rel).is_file(),
            listed_in_package_inventory: inventory.contains(surface.rel),
        })
        .collect::<Vec<_>>();
    rows.extend(discovered::rows(root, inventory));
    rows.sort_by(|left, right| left.role.cmp(&right.role).then(left.path.cmp(&right.path)));
    rows.dedup_by(|left, right| left.role == right.role && left.path == right.path);
    rows
}
