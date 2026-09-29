use crate::{
    boundaries, dependencies, inventory, metadata, model::Audit, modules, registry, syntax,
};
use std::{collections::BTreeMap, path::Path};

pub fn run(root: &Path, registry_path: &str, show_inventory: bool) -> serde_json::Value {
    let root = match root.canonicalize() {
        Ok(root) if root.is_dir() => root,
        other => {
            return serde_json::json!({
                "schema":"engineering-judgment.legibility.v1", "passed":false,
                "failures":[format!("project_root_invalid:{other:?}")],
                "governed_files":0, "exclusions":[], "limitations":[]
            });
        }
    };
    let root = root.as_path();
    let inventory = inventory::collect(root);
    let metadata = metadata::collect(root, &inventory);
    let mut failures = inventory.failures.clone();
    failures.extend(metadata.failures.clone());
    for path in inventory.files.keys().filter(|p| p.ends_with("Cargo.lock")) {
        if !metadata.verified_locks.contains(path) {
            failures.push(format!("generated_lock_unverified:{path}"));
        }
    }
    let mut reports = BTreeMap::new();
    let contexts = crate::source_context::contexts(&inventory.files);
    let bindings = crate::source_context::bindings::contexts(&inventory.files);
    let mut limitations = vec![
        "Exact clock/process-identity reads and std::thread::sleep in output producers are observations/local timing, not deterministic purity or latency guarantees; actual values require runtime records.".into(),
        "Portable adaptation of recovered UltraGoal source laws, not invocation of the legacy UltraGoal validator.".into(),
        "Rust semantic checks use conventional src module layouts; non-Rust executable sources fail with semantic_coverage_unsupported instead of receiving semantic approval.".into(),
        "Static AST enforcement does not prove architecture quality, domain-validation adequacy, runtime behavior or model benefit.".into(),
        "Authority checks use original production applicability and syntactic Result<Named,Named> closure; exact declared validation calls are checked, not their semantic adequacy.".into(),
    ];
    for (path, bytes) in &inventory.files {
        if !path.ends_with(".rs") {
            if crate::coverage::executable_source(path, bytes) {
                failures.push(format!("semantic_coverage_unsupported:{path}"));
            }
            continue;
        }
        let trusted = object_ancestor(path, &inventory.files);
        let context = contexts.get(path).cloned().unwrap_or_default();
        match std::str::from_utf8(bytes)
            .map_err(|e| e.to_string())
            .and_then(|s| {
                syntax::analyze_with_bindings(
                    path,
                    s,
                    trusted,
                    &context.anyhow,
                    &context.aliases,
                    &bindings.get(path).cloned().unwrap_or_default(),
                )
            }) {
            Ok(report) => {
                failures.extend(
                    report
                        .failures
                        .iter()
                        .map(|f| format!("source_syntax:{path}:{f}")),
                );
                limitations.extend(report.limitations.clone());
                reports.insert(path.clone(), report);
            }
            Err(error) => failures.push(format!("rust_parse:{path}:{error}")),
        }
    }
    let rust_files: Vec<_> = reports.keys().cloned().collect();
    let inclusions = crate::inclusions::check(root, &reports, &inventory);
    failures.extend(inclusions.failures);
    failures.extend(modules::check(root, &rust_files, &metadata.entrypoints).failures);
    let production_sources = modules::production_paths(root, &rust_files, &metadata.entrypoints);
    let test_only = production_sources.test_only;
    failures.extend(production_sources.failures);
    let mut production: BTreeMap<_, _> = reports
        .iter()
        .filter(|(path, _)| !test_only.contains(*path))
        .map(|(path, r)| (path.clone(), r.clone()))
        .collect();
    failures.extend(crate::generic_domains::enforce(
        &mut production,
        &inventory.files,
    ));
    for (path, report) in &production {
        reports.insert(path.clone(), report.clone());
    }
    match registry::load(&inventory, registry_path) {
        Ok(registry) => {
            failures.extend(registry::source_map(&registry.sources, &inventory));
            failures.extend(registry::commands(
                &registry.commands,
                &inventory,
                &metadata.entrypoints,
            ));
            failures.extend(dependencies::check_with_scope(
                &registry.dependencies,
                &metadata,
                &reports,
                &inventory,
                &registry.boundaries,
                &test_only,
            ));
            let outputs = crate::outputs::check(
                &registry.outputs,
                &registry.boundaries,
                &registry.dependencies,
                &production,
                &inventory,
            );
            failures.extend(outputs.failures);
            failures.extend(boundaries::check_with_outputs(
                &registry.boundaries,
                &production,
                &outputs.allowed,
            ));
        }
        Err(error) => {
            failures.push(error.to_string());
            failures.push("registry_setup_required:author exact source, dependency, command, boundary and output records; run with --inventory to inspect findings; no approval is generated".into());
            failures.extend(boundaries::check(&[], &production));
        }
    }
    failures.sort();
    failures.dedup();
    limitations.sort();
    limitations.dedup();
    let audit = Audit {
        schema: "engineering-judgment.legibility.v1".into(),
        passed: failures.is_empty(),
        governed_files: inventory.files.len(),
        failures,
        exclusions: inventory.exclusions.clone(),
        limitations,
    };
    let mut output = serde_json::to_value(audit).expect("typed audit serialization");
    if show_inventory {
        output["inventory"] = crate::diagnostics::inventory(&inventory, &metadata, &reports);
        output["inventory"]["test_only_files"] = serde_json::json!(test_only);
        output["inventory"]["data_inclusions"] = serde_json::json!(inclusions.bindings);
    }
    output
}

fn object_ancestor(path: &str, files: &BTreeMap<String, Vec<u8>>) -> bool {
    let mut parent = Path::new(path).parent();
    while let Some(directory) = parent {
        let candidate = directory.join("mod.rs").to_string_lossy().into_owned();
        if candidate != path
            && files
                .get(&candidate)
                .and_then(|b| std::str::from_utf8(b).ok())
                .is_some_and(syntax::object_contract_from_source)
        {
            return true;
        }
        parent = directory.parent();
    }
    false
}
