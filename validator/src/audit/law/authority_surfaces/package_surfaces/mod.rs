use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;
mod cargo;
mod document;
mod inventory_resource;
mod module_ownership;
mod proof_binding;
mod role;
mod row;
mod source;
pub(crate) use document::InventoryArtifactBinding;
pub(super) const CHECK_ID: &str = "purpose-backed-active-files";
const CLAIM_IMPACT: &str =
    "blocks completion,review,package,readiness,release,final_packet,update_goal";
const NARROW_RERUN: &str = "target/debug/ultragoal --root . typed-boundaries check --strict";
pub(super) fn failures(root: &Path, inventory: &BTreeSet<String>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let source_paths = source::actual_paths(root).unwrap_or_else(|err| {
        push(
            &mut out,
            "package-tree-scan",
            &format!("package source tree scan failed closed instead of hiding active surfaces: {err}"),
            "repair package tree readability or package inventory traversal, then rerun typed-boundaries check",
        );
        Vec::new()
    });
    let package_paths = source::actual_package_paths(root).unwrap_or_else(|err| {
        push(
            &mut out,
            "package-resource-scan",
            &format!("package resource tree scan failed closed instead of hiding active resources: {err}"),
            "repair package tree readability or package inventory traversal, then rerun typed-boundaries check",
        );
        Vec::new()
    });
    let rows = rows_from_paths(root, inventory, &source_paths);
    for rel in &source_paths {
        if let Err(err) = source::symbols(root, rel) {
            push(
                &mut out,
                &format!("source-symbol-read:{rel}"),
                &format!(
                    "package source symbols could not be read, which could hide active functions or types: {err}"
                ),
                "repair source file readability, then rerun typed-boundaries check",
            );
        }
    }
    let bins = cargo::bins(root);
    if bins.contains_key("ultragoal-validator") {
        push(
            &mut out,
            "binary:ultragoal-validator",
            "legacy validator binary duplicates canonical ultragoal CLI without a current external compatibility contract",
            "delete validator/src/bin/ultragoal-validator.rs or add a typed compatibility_alias contract with sunset and no canonical claim authority",
        );
    }
    if bins.len() > 1 && !bins.contains_key("ultragoal-validator") {
        push(
            &mut out,
            "binary:cargo-bin-duplicates",
            "multiple package binaries expose executable authority and no non-canonical compatibility contract is registered",
            "keep exactly one canonical CLI binary or classify the duplicate as compatibility_alias with contract and sunset",
        );
    }
    for rel in source_paths {
        if rel.starts_with("validator/src/bin/") && !bins.values().any(|path| path == &rel) {
            push(
                &mut out,
                &format!("binary-orphan:{rel}"),
                "binary source file exists without Cargo binary ownership",
                "remove the orphan binary file or declare exactly one canonical Cargo binary surface",
            );
        }
        if !inventory.contains(&rel) {
            push(
                &mut out,
                &format!("source-orphan:{rel}"),
                "package-owned Rust source exists outside package inventory",
                "add the source to package inventory or remove the unowned surface",
            );
        }
    }
    for rel in package_paths {
        if let Some(kind) = source::package_resource_kind(&rel)
            && !inventory.contains(&rel)
        {
            push(
                &mut out,
                &format!("{kind}-orphan:{rel}"),
                "package-owned resource exists outside package inventory",
                "add the resource to package inventory or remove the unowned surface",
            );
        }
    }
    for rel in inventory {
        if inventory_resource::live_proof_package_resource(rel) {
            push(
                &mut out,
                &format!("live-proof-package-resource:{rel}"),
                inventory_resource::live_proof_resource_reason(),
                inventory_resource::live_proof_resource_repair(),
            );
        }
        if rel.starts_with("validator/src/bin/") && !bins.values().any(|path| path == rel) {
            push(
                &mut out,
                &format!("binary-resource-orphan:{rel}"),
                "package inventory lists a binary source that is not an active Cargo binary",
                "remove the stale binary resource from package inventory",
            );
        }
    }
    if source::tree_contains(root, "validator/src/argument_parser", "\"package-digest\"")
        || source::contains(
            root,
            "validator/src/argument_parser.rs",
            "\"package-digest\"",
        )
    {
        push(
            &mut out,
            "command-alias:package-digest",
            "legacy package-digest command mirrors canonical package digest without a compatibility contract",
            "delete the alias or classify it as parser_boundary compatibility with claim limits and sunset",
        );
    }
    if source::contains(root, "validator/src/cli/usage.rs", "ultragoal-validator") {
        push(
            &mut out,
            "usage:ultragoal-validator",
            "help text exposes ultragoal-validator as an active command surface without contract",
            "remove the compatibility command language or add a typed alias contract",
        );
    }
    for surface in &rows {
        if row::dead_surface_preservation_label(&surface.path_or_symbol) {
            push(
                &mut out,
                &surface.surface_id,
                "surface name describes coverage/no-op preservation instead of product behavior",
                "delete the wrapper/test or rename it around the product behavior it actually validates",
            );
        }
        if let Some(reason) = row::contract_failure(surface) {
            push(
                &mut out,
                &surface.surface_id,
                reason,
                "bind the active surface to product-role semantics plus fixture and receipt reconciliation before it can support claims",
            );
        }
    }
    out
}

pub(super) fn value(root: &Path, inventory: &BTreeSet<String>) -> Value {
    let rows = rows(root, inventory);
    document::full(rows)
}

pub(super) fn summary_value(
    root: &Path,
    inventory: &BTreeSet<String>,
    artifact: &InventoryArtifactBinding,
) -> Value {
    let rows = rows(root, inventory);
    document::summary(rows, artifact)
}

pub(super) fn rows(root: &Path, inventory: &BTreeSet<String>) -> Vec<row::PackageSurfaceRow> {
    let source_paths = source::actual_paths(root).unwrap_or_default();
    rows_from_paths(root, inventory, &source_paths)
}

fn rows_from_paths(
    root: &Path,
    inventory: &BTreeSet<String>,
    source_paths: &[String],
) -> Vec<row::PackageSurfaceRow> {
    let bins = cargo::bins(root);
    let mut rows = Vec::new();
    for (name, path) in bins {
        rows.push(row::from_parts(
            "binary",
            &path,
            if name == "ultragoal" {
                "canonical product CLI executable"
            } else {
                "compatibility CLI executable alias"
            },
            if name == "ultragoal" {
                "canonical"
            } else {
                "compatibility_alias"
            },
            if name == "ultragoal" {
                None
            } else {
                Some("binary:ultragoal")
            },
        ));
    }
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
            rows.push(row::package_resource(rel));
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

fn push(out: &mut Vec<(String, String)>, surface: &str, why: &str, repair: &str) {
    out.push((
        CHECK_ID.to_string(),
        format!(
            "failure_class=purpose_backed_surface_violation;surface={surface};why_failed={why};claim_impact={CLAIM_IMPACT};smallest_repair={repair};narrow_rerun={NARROW_RERUN}"
        ),
    ));
}

#[cfg(test)]
pub(crate) fn failures_for_test(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<(String, String)> {
    failures(root, inventory)
}
