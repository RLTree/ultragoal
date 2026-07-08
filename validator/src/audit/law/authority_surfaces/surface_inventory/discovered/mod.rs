use super::AuthoritySurfaceInventoryRow;
use std::collections::BTreeSet;
use std::path::Path;

mod artifact_authority;
mod law_catalog_authority;
mod source_authority;

pub(super) fn rows(root: &Path, inventory: &BTreeSet<String>) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut rows = Vec::new();
    rows.extend(source_authority::rows(root, inventory));
    rows.extend(law_catalog_authority::rows(root, inventory));
    rows.extend(artifact_authority::rows(root, inventory));
    rows
}

fn authority_row(
    role: &str,
    path: &str,
    package_inventory_required: bool,
    root: &Path,
    inventory: &BTreeSet<String>,
) -> AuthoritySurfaceInventoryRow {
    AuthoritySurfaceInventoryRow {
        role: role.to_string(),
        path: path.to_string(),
        package_inventory_required,
        existence_required: true,
        exists_on_disk: root.join(path).is_file(),
        listed_in_package_inventory: inventory.contains(path),
    }
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, out);
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}
