fn candidate_components(
    entries: &BTreeMap<String, InventoryEntry>,
    findings: &mut Vec<InventoryFinding>,
) {
    for entry in entries
        .values()
        .filter(|entry| entry.active_status == ActiveStatus::Candidate)
    {
        findings.push(InventoryFinding::warning(
            "candidate_component_not_active",
            Some(&entry.stable_id),
            Some(&entry.relative_path),
            "compiled component exists but is not an adopted live product route".to_owned(),
        ));
    }
}

pub(crate) fn reconcile(
    root: &Path,
    groups: Vec<Vec<InventoryEntry>>,
    mut findings: Vec<InventoryFinding>,
    routing: &RoutingData,
    verified_pending_authority: &BTreeSet<String>,
) -> (Vec<InventoryEntry>, Vec<InventoryFinding>) {
    let duplicate_stable_id_conflicts = duplicate_stable_ids(&groups);
    let duplicate_path_conflicts = duplicate_paths(&groups, &mut findings);
    let mut entries = BTreeMap::new();
    for group in groups {
        for entry in group {
            merge_one(&mut entries, entry, &mut findings);
        }
    }
    routing.apply(
        &mut entries,
        &mut findings,
        &duplicate_stable_id_conflicts,
        &duplicate_path_conflicts,
    );
    missing_required(&mut entries, &mut findings);
    unresolved_references(root, &entries, &mut findings);
    candidate_components(&entries, &mut findings);
    parallel_authority(&entries, &mut findings, verified_pending_authority);
    let mut entries = entries.into_values().collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        left.stable_id
            .cmp(&right.stable_id)
            .then(left.relative_path.cmp(&right.relative_path))
    });
    findings.sort();
    findings.dedup();
    (entries, findings)
}

impl AuthorityCatalog {
    pub fn closure_status(&self) -> InventoryClosureStatus {
        let mut blockers = BTreeMap::new();
        let mut open_obligations = BTreeMap::new();
        for finding in self.findings() {
            let counts = match finding.closure_disposition() {
                InventoryFindingDisposition::Blocking => &mut blockers,
                InventoryFindingDisposition::OpenMigrationObligation => &mut open_obligations,
                InventoryFindingDisposition::Informational => continue,
            };
            *counts.entry(finding.code.clone()).or_insert(0) += 1;
        }
        InventoryClosureStatus::new(
            self.catalog_id().to_owned(),
            self.context_id().to_owned(),
            blockers,
            open_obligations,
        )
    }
}
