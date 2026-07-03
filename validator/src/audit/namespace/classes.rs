use crate::audit::namespace::law::path_rules::str_field;
use serde_json::Value;
use std::path::Path;

const EXPECTED_SCHEMA: &str = "harness-ultragoal.namespace-class-registry.v1";
const CLAIM_IMPACT: &str = "classifies_surface_without_raising_claim_ceiling";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NamespaceClassKind {
    BuildManifest,
    Connector,
    Documentation,
    Example,
    ExecutableScript,
    ExternalCompatibility,
    FixtureCatalog,
    RepoSource,
    GeneratedArtifact,
    PublicDistribution,
    Template,
}

pub(crate) fn value_failures(root: &Path, value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if str_field(value, "schema") != EXPECTED_SCHEMA {
        out.push("namespace_class_registry_schema_invalid".to_string());
    }
    let Some(rows) = value.get("classes").and_then(Value::as_array) else {
        out.push("namespace_class_registry_classes_missing".to_string());
        return out;
    };
    if rows.is_empty() {
        out.push("namespace_class_registry_classes_empty".to_string());
    }
    for row in rows {
        let id = str_field(row, "id");
        let kind = parse_kind(&str_field(row, "kind"));
        required_field_failures(row, &id, &mut out);
        waiver_field_failures(row, &id, &mut out);
        if kind.is_none() {
            out.push(format!("namespace_class_kind_invalid:{id}"));
        }
        require_existing(
            root,
            row,
            "authority_path",
            "namespace_class_authority_missing",
            &id,
            &mut out,
        );
        out.extend(surface_failures(row, &id, kind));
    }
    out
}

pub(crate) fn legacy_surface_failures(root: &Path, manifest_paths: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for rel in [
        "docs/namespace-law-exceptions.json",
        "schemas/namespace-law-exceptions.schema.json",
    ] {
        if root.join(rel).exists() {
            out.push(format!(
                "namespace_waiver_surface_present:path={rel};repair=delete_legacy_namespace_exception_surface;claims=completion,review,package,readiness,release,cli_self_law,update_goal"
            ));
        }
        if manifest_paths.iter().any(|path| path == rel) {
            out.push(format!(
                "namespace_waiver_surface_listed:path={rel};repair=remove_legacy_namespace_exception_surface_from_package_inventory;claims=completion,review,package,readiness,release,cli_self_law,update_goal"
            ));
        }
    }
    out
}

pub(crate) fn resolution_failures(value: &Value, paths: &[String]) -> Vec<String> {
    let rows = value
        .get("classes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    for path in paths {
        let matches = rows
            .iter()
            .filter(|row| path_matches_class(row, path))
            .filter_map(|row| row.get("id").and_then(Value::as_str))
            .collect::<Vec<_>>();
        match matches.len() {
            1 => {}
            0 => out.push(format!("namespace_class_resolution_missing:{path}")),
            _ => out.push(format!(
                "namespace_class_resolution_ambiguous:{path}:{}",
                matches.join(",")
            )),
        }
    }
    out
}

fn required_field_failures(row: &Value, id: &str, out: &mut Vec<String>) {
    let surface_globs = row
        .get("surface_globs")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    if id.is_empty()
        || str_field(row, "description").is_empty()
        || str_field(row, "authority").is_empty()
        || str_field(row, "claim_ceiling_impact") != CLAIM_IMPACT
        || row
            .get("maximal_factoring_required")
            .and_then(Value::as_bool)
            != Some(true)
        || row.get("waiver_allowed").and_then(Value::as_bool) != Some(false)
        || !surface_globs
    {
        out.push(format!("namespace_class_untyped:{id}"));
    }
}

fn waiver_field_failures(row: &Value, id: &str, out: &mut Vec<String>) {
    for field in [
        "exception_type",
        "directory",
        "prefix",
        "applies_to",
        "reason",
        "contract_path",
        "generator_or_catalog_path",
    ] {
        if row.get(field).is_some() {
            out.push(format!("namespace_class_waiver_field_present:{id}:{field}"));
        }
    }
}

fn parse_kind(raw: &str) -> Option<NamespaceClassKind> {
    match raw {
        "build_manifest" => Some(NamespaceClassKind::BuildManifest),
        "connector" => Some(NamespaceClassKind::Connector),
        "documentation" => Some(NamespaceClassKind::Documentation),
        "example" => Some(NamespaceClassKind::Example),
        "executable_script" => Some(NamespaceClassKind::ExecutableScript),
        "external_compatibility" => Some(NamespaceClassKind::ExternalCompatibility),
        "fixture_catalog" => Some(NamespaceClassKind::FixtureCatalog),
        "repo_source" => Some(NamespaceClassKind::RepoSource),
        "generated_artifact" => Some(NamespaceClassKind::GeneratedArtifact),
        "public_distribution" => Some(NamespaceClassKind::PublicDistribution),
        "template" => Some(NamespaceClassKind::Template),
        _ => None,
    }
}

fn surface_failures(row: &Value, id: &str, kind: Option<NamespaceClassKind>) -> Vec<String> {
    let globs = row
        .get("surface_globs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    for glob in globs {
        if glob.starts_with('/') || glob.contains("..") {
            out.push(format!("namespace_class_surface_invalid:{id}:{glob}"));
        }
        if glob.starts_with("validator/src")
            && matches!(
                kind,
                Some(NamespaceClassKind::GeneratedArtifact | NamespaceClassKind::FixtureCatalog)
            )
        {
            out.push(format!(
                "namespace_class_generated_for_hand_authored_source:{id}:{glob}"
            ));
        }
        if glob == "validator/src/*"
            || glob == "validator/src/*.rs"
            || glob == "validator/src/internal*"
            || glob.starts_with("validator/src/internal")
        {
            out.push(format!(
                "namespace_class_broad_source_glob_requires_direct_topology_check:{id}:{glob}"
            ));
        }
    }
    out
}

fn path_matches_class(row: &Value, path: &str) -> bool {
    row.get("surface_globs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|glob| glob_match(glob, path))
}

fn glob_match(glob: &str, path: &str) -> bool {
    if glob == path {
        return true;
    }
    if let Some(prefix) = glob.strip_suffix("/**") {
        return path.starts_with(&format!("{prefix}/"));
    }
    if let Some((prefix, suffix)) = glob.split_once("/**/*") {
        return path.starts_with(&format!("{prefix}/")) && path.ends_with(suffix);
    }
    if let Some((prefix, suffix)) = glob.split_once("/**/") {
        return path.starts_with(&format!("{prefix}/")) && path.ends_with(suffix);
    }
    false
}

fn require_existing(
    root: &Path,
    row: &Value,
    key: &str,
    error: &str,
    id: &str,
    out: &mut Vec<String>,
) {
    let rel = str_field(row, key);
    if rel.is_empty()
        || crate::package::inventory::resolve(root, &rel)
            .map(|path| !path.exists())
            .unwrap_or(true)
    {
        out.push(format!("{error}:{id}"));
    }
}
