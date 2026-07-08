use std::collections::BTreeSet;
use std::path::Path;

#[path = "../observability_package_surfaces.rs"]
mod observability_package_surfaces;
#[path = "../package_surfaces/requirements.rs"]
mod package_surface_requirements;

#[derive(Clone, Copy)]
pub(super) struct RequiredSurface {
    pub(super) role: &'static str,
    pub(super) rel: &'static str,
    pub(super) package_inventory_required: bool,
    pub(super) existence_required: bool,
}

pub(super) fn failures(root: &Path, inventory: &BTreeSet<String>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for surface in super::surface_inventory::rows(root, inventory) {
        if surface.existence_required && !surface.exists_on_disk {
            out.push((
                "authority-source-binding".to_string(),
                format!(
                    "authority_surface_missing:role={}:path={}",
                    surface.role, surface.path
                ),
            ));
        }
        if surface.package_inventory_required && !surface.listed_in_package_inventory {
            out.push((
                "authority-source-binding".to_string(),
                format!(
                    "authority_surface_not_in_package_inventory:role={}:path={}",
                    surface.role, surface.path
                ),
            ));
        }
    }
    out
}

pub(super) fn required_surfaces() -> Vec<RequiredSurface> {
    let mut surfaces = super::inventory_required_surfaces::REQUIRED_SURFACES.to_vec();
    surfaces.extend_from_slice(package_surface_requirements::REQUIRED_SURFACES);
    surfaces.extend_from_slice(observability_package_surfaces::REQUIRED_SURFACES);
    surfaces
}

#[cfg(test)]
pub(crate) fn required_surfaces_for_test() -> Vec<(&'static str, &'static str, bool)> {
    required_surfaces()
        .into_iter()
        .map(surface_for_test)
        .collect()
}

#[cfg(test)]
fn surface_for_test(surface: RequiredSurface) -> (&'static str, &'static str, bool) {
    (
        surface.role,
        surface.rel,
        surface.package_inventory_required,
    )
}
