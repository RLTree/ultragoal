use super::*;

pub(crate) fn inventory_findings(
    inputs: &BoundInputs,
    catalog: &DependencyActionCatalog,
    output: &mut Vec<Finding>,
    fatal: &mut Vec<String>,
) {
    for observation in &inputs.inventory_findings {
        let policies = catalog
            .spec()
            .inventory_policies
            .iter()
            .filter(|policy| policy.code == observation.code)
            .collect::<Vec<_>>();
        if policies.len() != 1 {
            fatal.push(format!(
                "inventory-policy-count:{}:{}",
                observation.code,
                policies.len()
            ));
            continue;
        }
        let policy = policies[0];
        output.push(finding(
            observation.code.clone(),
            match observation.severity {
                InventorySeverity::Error => FindingSeverity::Error,
                InventorySeverity::Warning => FindingSeverity::Warning,
                InventorySeverity::Info => FindingSeverity::Info,
            },
            FindingSource::AuthorityCatalog {
                catalog_id: inputs.authority_catalog_id.clone(),
                code: observation.code.clone(),
            },
            Scope {
                surface: policy.scope_surface.clone(),
                relative_path: observation.relative_path.clone(),
            },
            observation.entry_id.iter().cloned().collect(),
            observation.cause.clone(),
            policy.repair.clone(),
            policy.ceiling_reductions.clone(),
        ));
    }
}

pub(crate) fn dependency_findings(
    catalog: &DependencyActionCatalog,
    output: &mut Vec<Finding>,
    fatal: &mut Vec<String>,
) -> BTreeMap<String, Option<DependencyStatus>> {
    let mut groups: BTreeMap<&str, Vec<&super::super::catalog::DependencyFact>> = BTreeMap::new();
    for fact in &catalog.spec().dependencies {
        groups.entry(&fact.dependency_id).or_default().push(fact);
    }
    let mut states = BTreeMap::new();
    for (dependency_id, facts) in groups {
        let statuses = facts
            .iter()
            .map(|fact| fact.status)
            .collect::<BTreeSet<_>>();
        if statuses.len() > 1 {
            states.insert(dependency_id.to_owned(), None);
            let reductions = facts
                .iter()
                .flat_map(|fact| fact.ceiling_reductions.clone())
                .collect();
            let cause = facts
                .iter()
                .map(|fact| {
                    format!(
                        "{}:{}:{:?}",
                        fact.observation_id,
                        authority_name(fact.authority),
                        fact.status
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            output.push(finding(
                "contradictory-dependency",
                FindingSeverity::Error,
                FindingSource::DependencyCatalog {
                    catalog_id: catalog.catalog_id().to_owned(),
                    observation_id: "multiple".to_owned(),
                },
                Scope {
                    surface: "dependency-graph".to_owned(),
                    relative_path: None,
                },
                BTreeSet::from([dependency_id.to_owned()]),
                cause,
                contradiction_repair(dependency_id),
                reductions,
            ));
            continue;
        }
        let status = *statuses.iter().next().expect("nonempty dependency facts");
        states.insert(dependency_id.to_owned(), Some(status));
        if status == DependencyStatus::Satisfied {
            continue;
        }
        for fact in facts {
            let Some(repair) = fact.repair.clone() else {
                fatal.push(format!("missing-repair:{dependency_id}"));
                continue;
            };
            output.push(finding(
                dependency_code(status),
                dependency_severity(status),
                FindingSource::DependencyCatalog {
                    catalog_id: catalog.catalog_id().to_owned(),
                    observation_id: fact.observation_id.clone(),
                },
                fact.scope.clone(),
                BTreeSet::from([dependency_id.to_owned()]),
                fact.cause.clone(),
                repair,
                fact.ceiling_reductions.clone(),
            ));
        }
    }
    states
}
