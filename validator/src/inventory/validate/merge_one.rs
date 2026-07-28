fn merge_one(
    entries: &mut BTreeMap<String, InventoryEntry>,
    mut incoming: InventoryEntry,
    findings: &mut Vec<InventoryFinding>,
) {
    incoming.normalize();
    let Some(existing) = entries.get(&incoming.stable_id).cloned() else {
        entries.insert(incoming.stable_id.clone(), incoming);
        return;
    };
    let existing_required = matches!(
        existing.active_status,
        ActiveStatus::Required | ActiveStatus::Missing
    );
    let incoming_required = matches!(
        incoming.active_status,
        ActiveStatus::Required | ActiveStatus::Missing
    );
    let existing_semantic = existing.relative_path.starts_with("@semantic/");
    let incoming_semantic = incoming.relative_path.starts_with("@semantic/");
    if existing_required ^ incoming_required || existing_semantic ^ incoming_semantic {
        let (expected, mut actual) = if existing_semantic || existing_required {
            (existing, incoming)
        } else {
            (incoming, existing)
        };
        if !expected.relative_path.starts_with("@semantic/")
            && expected.relative_path != actual.relative_path
        {
            findings.push(InventoryFinding::error(
                "renamed_required_component",
                Some(&actual.stable_id),
                Some(&actual.relative_path),
                format!(
                    "expected path {}, found {}",
                    expected.relative_path, actual.relative_path
                ),
            ));
        }
        actual.owner_role = expected.owner_role;
        actual.authority_state = expected.authority_state;
        actual.references.extend(expected.references);
        actual
            .input_provenance
            .push(format!("required-path:{}", expected.relative_path));
        actual.normalize();
        entries.insert(actual.stable_id.clone(), actual);
        return;
    }
    findings.push(InventoryFinding::error(
        "duplicate_stable_id",
        Some(&incoming.stable_id),
        Some(&incoming.relative_path),
        format!("also defined at {}", existing.relative_path),
    ));
}

fn missing_required(
    entries: &mut BTreeMap<String, InventoryEntry>,
    findings: &mut Vec<InventoryFinding>,
) {
    for entry in entries.values_mut() {
        if entry.active_status == ActiveStatus::Required {
            entry.active_status = ActiveStatus::Missing;
            findings.push(InventoryFinding::error(
                "missing_required_component",
                Some(&entry.stable_id),
                Some(&entry.relative_path),
                "required contract component was not discovered".to_owned(),
            ));
        }
    }
}

fn schema_reference_exists(root: &Path, entry: &InventoryEntry, reference: &str) -> bool {
    if reference.starts_with('#') || reference.contains("://") {
        return true;
    }
    let path_part = reference.split('#').next().unwrap_or_default();
    if path_part.is_empty() {
        return true;
    }
    let parent = Path::new(&entry.relative_path)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    root.join(parent)
        .join(path_part)
        .canonicalize()
        .is_ok_and(|resolved| resolved.starts_with(root) && resolved.is_file())
}

fn unresolved_references(
    root: &Path,
    entries: &BTreeMap<String, InventoryEntry>,
    findings: &mut Vec<InventoryFinding>,
) {
    let ids = entries.keys().map(String::as_str).collect::<BTreeSet<_>>();
    for entry in entries.values() {
        if entry.kind == "generated-surface"
            && let Some(generator) = entry.generator.as_deref()
            && !ids.contains(generator)
        {
            findings.push(InventoryFinding::error(
                "unresolved_generator",
                Some(&entry.stable_id),
                Some(&entry.relative_path),
                "generated surface names an unknown Harness tool".to_owned(),
            ));
        }
        for reference in &entry.references {
            let semantic = [
                "PS-",
                "HCT-",
                "CL-",
                "REQ-",
                "SKILL:",
                "AGENT:",
                "COMMAND:",
                "CONTRACT-REGISTRY:",
                "API:",
                "J-",
                "N0",
                "N1",
                "WS-",
                "SRC-",
                "TRUTH-LAYER:",
            ]
            .iter()
            .any(|prefix| reference.starts_with(prefix));
            let resolved = if semantic {
                ids.contains(reference.as_str())
            } else if entry.kind == "schema" {
                schema_reference_exists(root, entry, reference)
            } else {
                true
            };
            if !resolved {
                findings.push(InventoryFinding::error(
                    "unresolved_reference",
                    Some(&entry.stable_id),
                    Some(&entry.relative_path),
                    format!("unresolved reference {reference}"),
                ));
            }
        }
    }
}

fn parallel_authority(
    entries: &BTreeMap<String, InventoryEntry>,
    findings: &mut Vec<InventoryFinding>,
    verified_pending_authority: &BTreeSet<String>,
) {
    for legacy in entries.values().filter(|entry| {
        entry.authority_state == AuthorityState::Legacy
            && entry.active_status == ActiveStatus::Active
    }) {
        if verified_pending_authority.contains(&legacy.stable_id) {
            findings.push(InventoryFinding::warning(
                "sole_current_authority_pending_migration",
                Some(&legacy.stable_id),
                Some(&legacy.relative_path),
                "one active legacy implementation is verified and its successor is definition-only; migration and retirement remain open"
                    .to_owned(),
            ));
        } else {
            findings.push(InventoryFinding::error(
                "parallel_authority",
                Some(&legacy.stable_id),
                Some(&legacy.relative_path),
                "legacy surface remains authoritative without an adopted disposition".to_owned(),
            ));
        }
    }
}
