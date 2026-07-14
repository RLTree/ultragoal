use super::{CHECK_ID, cargo, generated, inventory_resource, row, rows, source};
use std::collections::BTreeSet;
use std::path::Path;

const CLAIM_IMPACT: &str =
    "blocks completion,review,package,readiness,release,final_packet,update_goal";
const NARROW_RERUN: &str =
    "target/debug/ultragoal --root . check strict --claim cli-self-law-compliance";

pub(in crate::audit::law::authority_surfaces) fn failures(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let source_paths = source::actual_paths(root).unwrap_or_else(|err| {
        push(&mut out, "package-tree-scan", &format!(
            "package source tree scan failed closed instead of hiding active surfaces: {err}"
        ), "repair package tree readability or package inventory traversal, then rerun the canonical strict self-law claim");
        Vec::new()
    });
    let package_paths = source::actual_package_paths(root).unwrap_or_else(|err| {
        push(&mut out, "package-resource-scan", &format!(
            "package resource tree scan failed closed instead of hiding active resources: {err}"
        ), "repair package tree readability or package inventory traversal, then rerun the canonical strict self-law claim");
        Vec::new()
    });
    let generated = generated::State::build(root, inventory);
    for (surface, why) in generated.failures() {
        push(
            &mut out,
            &surface,
            &why,
            "adopt a valid digest-bound generated disposition or repair the canonical projection",
        );
    }
    let rows = rows::from_paths(root, inventory, &source_paths, &generated);
    validate_source_symbols(root, &source_paths, &mut out);
    let bins = cargo::bins(root);
    validate_binaries(&bins, &mut out);
    validate_source_ownership(&source_paths, inventory, &bins, &mut out);
    validate_resource_ownership(&package_paths, inventory, &mut out);
    validate_inventory_resources(inventory, &bins, &mut out);
    validate_command_aliases(root, &mut out);
    validate_rows(&rows, &mut out);
    out
}

fn validate_source_symbols(root: &Path, source_paths: &[String], out: &mut Vec<(String, String)>) {
    for rel in source_paths {
        if let Err(err) = source::symbols(root, rel) {
            push(
                out,
                &format!("source-symbol-read:{rel}"),
                &format!(
                    "package source symbols could not be read, which could hide active functions or types: {err}"
                ),
                "repair source file readability, then rerun the canonical strict self-law claim",
            );
        }
    }
}

fn validate_binaries(
    bins: &std::collections::BTreeMap<String, String>,
    out: &mut Vec<(String, String)>,
) {
    if bins.contains_key("ultragoal-validator") {
        push(
            out,
            "binary:ultragoal-validator",
            "legacy validator binary duplicates canonical ultragoal CLI without a current external compatibility contract",
            "delete validator/src/bin/ultragoal-validator.rs or add a typed compatibility_alias contract with sunset and no canonical claim authority",
        );
    }
    if bins.len() > 1 && !bins.contains_key("ultragoal-validator") {
        push(
            out,
            "binary:cargo-bin-duplicates",
            "multiple package binaries expose executable authority and no non-canonical compatibility contract is registered",
            "keep exactly one canonical CLI binary or classify the duplicate as compatibility_alias with contract and sunset",
        );
    }
}

fn validate_source_ownership(
    source_paths: &[String],
    inventory: &BTreeSet<String>,
    bins: &std::collections::BTreeMap<String, String>,
    out: &mut Vec<(String, String)>,
) {
    for rel in source_paths {
        if rel.starts_with("validator/src/bin/") && !bins.values().any(|path| path == rel) {
            push(
                out,
                &format!("binary-orphan:{rel}"),
                "binary source file exists without Cargo binary ownership",
                "remove the orphan binary file or declare exactly one canonical Cargo binary surface",
            );
        }
        if !inventory.contains(rel) {
            push(
                out,
                &format!("source-orphan:{rel}"),
                "package-owned Rust source exists outside package inventory",
                "add the source to package inventory or remove the unowned surface",
            );
        }
    }
}

fn validate_resource_ownership(
    package_paths: &[String],
    inventory: &BTreeSet<String>,
    out: &mut Vec<(String, String)>,
) {
    for rel in package_paths {
        if let Some(kind) = source::package_resource_kind(rel)
            && !inventory.contains(rel)
        {
            push(
                out,
                &format!("{kind}-orphan:{rel}"),
                "package-owned resource exists outside package inventory",
                "add the resource to package inventory or remove the unowned surface",
            );
        }
    }
}

fn validate_inventory_resources(
    inventory: &BTreeSet<String>,
    bins: &std::collections::BTreeMap<String, String>,
    out: &mut Vec<(String, String)>,
) {
    for rel in inventory {
        if inventory_resource::live_proof_package_resource(rel) {
            push(
                out,
                &format!("live-proof-package-resource:{rel}"),
                inventory_resource::live_proof_resource_reason(),
                inventory_resource::live_proof_resource_repair(),
            );
        }
        if rel.starts_with("validator/src/bin/") && !bins.values().any(|path| path == rel) {
            push(
                out,
                &format!("binary-resource-orphan:{rel}"),
                "package inventory lists a binary source that is not an active Cargo binary",
                "remove the stale binary resource from package inventory",
            );
        }
    }
}

fn validate_command_aliases(root: &Path, out: &mut Vec<(String, String)>) {
    if source::tree_contains(root, "validator/src/argument_parser", "\"package-digest\"")
        || source::contains(
            root,
            "validator/src/argument_parser.rs",
            "\"package-digest\"",
        )
    {
        push(
            out,
            "command-alias:package-digest",
            "legacy package-digest command mirrors canonical package digest without a compatibility contract",
            "delete the alias or classify it as parser_boundary compatibility with claim limits and sunset",
        );
    }
    if source::contains(root, "validator/src/cli/usage.rs", "ultragoal-validator") {
        push(
            out,
            "usage:ultragoal-validator",
            "help text exposes ultragoal-validator as an active command surface without contract",
            "remove the compatibility command language or add a typed alias contract",
        );
    }
}

fn validate_rows(rows: &[row::PackageSurfaceRow], out: &mut Vec<(String, String)>) {
    for surface in rows {
        if row::dead_surface_preservation_label(&surface.path_or_symbol) {
            push(
                out,
                &surface.surface_id,
                "surface name describes coverage/no-op preservation instead of product behavior",
                "delete the wrapper/test or rename it around the product behavior it actually validates",
            );
        }
        if let Some(reason) = row::contract_failure(surface) {
            push(
                out,
                &surface.surface_id,
                reason,
                "bind the active surface to product-role semantics plus fixture and receipt reconciliation before it can support claims",
            );
        }
    }
}

fn push(out: &mut Vec<(String, String)>, surface: &str, why: &str, repair: &str) {
    out.push((CHECK_ID.to_string(), format!(
        "failure_class=purpose_backed_surface_violation;surface={surface};why_failed={why};claim_impact={CLAIM_IMPACT};smallest_repair={repair};narrow_rerun={NARROW_RERUN}"
    )));
}

#[cfg(test)]
pub(crate) fn failures_for_test(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<(String, String)> {
    failures(root, inventory)
}
