use super::{cargo, generated, module_ownership, row, source};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn from_paths(
    root: &Path,
    inventory: &BTreeSet<String>,
    source_paths: &[String],
    generated: &generated::State,
) -> Vec<row::PackageSurfaceRow> {
    let mut rows = binary_rows(root);
    for rel in source_paths {
        if rel.starts_with("validator/src/bin/") {
            continue;
        }
        let module_test_only = module_ownership::cfg_test_module_file(root, rel);
        rows.push(row::source_module(rel, module_test_only));
        rows.extend(
            source::symbols(root, rel)
                .unwrap_or_default()
                .into_iter()
                .map(|symbol| row::source_symbol(rel, &symbol, module_test_only)),
        );
    }
    for rel in inventory {
        if !rel.ends_with(".rs") {
            rows.push(
                if crate::package::inventory::generated_disposition::generated_path(rel) {
                    generated.row(rel)
                } else {
                    row::package_resource(rel)
                },
            );
        }
    }
    rows.sort_by(|left, right| {
        left.surface_kind
            .cmp(&right.surface_kind)
            .then(left.path_or_symbol.cmp(&right.path_or_symbol))
    });
    rows.dedup_by(|left, right| {
        left.surface_kind == right.surface_kind && left.path_or_symbol == right.path_or_symbol
    });
    rows
}

fn binary_rows(root: &Path) -> Vec<row::PackageSurfaceRow> {
    cargo::bins(root)
        .into_iter()
        .map(|(name, path)| {
            let canonical = name == "ultragoal";
            row::from_parts(
                "binary",
                &path,
                if canonical {
                    "canonical product CLI executable"
                } else {
                    "compatibility CLI executable alias"
                },
                if canonical {
                    "canonical"
                } else {
                    "compatibility_alias"
                },
                (!canonical).then_some("binary:ultragoal"),
            )
        })
        .collect()
}
