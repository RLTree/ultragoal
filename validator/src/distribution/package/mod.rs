mod archive;
mod manifest;
mod manifest_bind;
mod materialize;
mod output;
mod plan;
mod snapshot;
mod source;
mod spec;

use std::collections::BTreeSet;

pub use materialize::{
    ExpectedTree, MaterializeEffects, MaterializeTransaction, TreeObject, TreeObjectKind,
    materialize_package, reconcile as reconcile_materialized_tree, rollback_materialization,
    tree_sha256,
};
pub use output::{
    PackageArtifactBinding, PackageArtifactTransaction, publish_package_artifact,
    reconcile_package_artifact, recover_package_artifact, rollback_package_artifact,
};
pub use plan::{
    PackageEffects, PackageEntry, PackagePlan, PackageSnapshot, build_package, plan_package,
    plan_package_from_inventory, verify_package,
};
include_production_package_module!();
pub use product::{
    ProductionPackageArtifact, ProductionPackageError, ProductionPackageErrorId,
    ProductionPackageSession, capture_product_package, verify_product_package,
};
pub use spec::PackageRole;

fn insert_prefix_free_path(paths: &mut BTreeSet<String>, path: &str) -> bool {
    let folded = path.to_ascii_lowercase();
    if paths.iter().any(|existing| {
        is_same_or_ancestor(existing, &folded) || is_same_or_ancestor(&folded, existing)
    }) {
        return false;
    }
    paths.insert(folded)
}

fn is_same_or_ancestor(ancestor: &str, path: &str) -> bool {
    ancestor == path
        || (path.starts_with(ancestor) && path.as_bytes().get(ancestor.len()) == Some(&b'/'))
}

#[cfg(test)]
mod tests {
    use super::insert_prefix_free_path;
    use std::collections::BTreeSet;

    #[test]
    fn prefix_free_paths_are_order_independent_case_folded_and_segment_aware() {
        for (first, second) in [
            ("assets", "assets/icon.svg"),
            ("assets/icon.svg", "assets"),
            ("Assets", "assets/icon.svg"),
            ("skills/x", "SKILLS/X/SKILL.md"),
        ] {
            let mut paths = BTreeSet::new();
            assert!(insert_prefix_free_path(&mut paths, first));
            assert!(!insert_prefix_free_path(&mut paths, second));
        }
        let mut siblings = BTreeSet::new();
        for path in [
            "assets/a",
            "assets/ab",
            "skills/a/SKILL.md",
            "skills/ab/SKILL.md",
        ] {
            assert!(insert_prefix_free_path(&mut siblings, path));
        }
    }
}
