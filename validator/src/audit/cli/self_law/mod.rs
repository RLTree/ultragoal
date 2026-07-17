use std::collections::BTreeMap;

mod composition;
mod contracts;
mod inventory;

pub(crate) use contracts::{
    CliSelfLawCheckError, CliSelfLawCheckRequest, CliSelfLawCheckResponse, CliSelfLawFinding,
};

pub(in crate::audit) use composition::append_package_text_checks;
pub(in crate::audit) use inventory::append as append_inventory_checks;

const CLAIM_ID: &str = "cli-self-law-compliance";

pub(crate) fn check(
    request: CliSelfLawCheckRequest<'_>,
) -> Result<CliSelfLawCheckResponse, CliSelfLawCheckError> {
    let scheduler = crate::scheduler::SchedulerConfig::from_jobs(Some(request.jobs))
        .map_err(|_| CliSelfLawCheckError::InvalidParallelism)?;
    let store = crate::schema_catalog::load(request.root);
    let mut failures = BTreeMap::new();
    for failure in &store.errors {
        push(
            &mut failures,
            "schema-valid",
            format!("schema_catalog:{failure}"),
        );
    }
    let schema = crate::audit::package::schema::validation::mapped(request.root, &store, scheduler);
    for failure in schema.failures {
        push(&mut failures, "schema-valid", failure);
    }
    inventory::append(request.root, &mut failures);
    crate::audit::red::catalog::check(request.root, &store, &mut failures);
    composition::append_base_checks(request.root, &store, &mut failures);

    let current_details = failures
        .iter()
        .flat_map(|(check, details)| {
            details
                .iter()
                .map(move |detail| format!("{check}:{detail}"))
        })
        .collect::<Vec<_>>();
    let current = BTreeMap::from([(CLAIM_ID.to_string(), current_details)]);
    for failure in crate::audit::mandatory::law::surfaces::package_failures_for_current_law(
        request.root,
        CLAIM_ID,
        &current,
    ) {
        push(&mut failures, CLAIM_ID, failure);
    }
    Ok(stable_response(failures))
}

fn stable_response(failures: BTreeMap<String, Vec<String>>) -> CliSelfLawCheckResponse {
    let mut findings = failures
        .into_iter()
        .flat_map(|(check_id, details)| {
            details.into_iter().map(move |detail| CliSelfLawFinding {
                check_id: check_id.clone(),
                detail,
            })
        })
        .collect::<Vec<_>>();
    findings.sort();
    findings.dedup();
    CliSelfLawCheckResponse::new(findings)
}

fn push(failures: &mut BTreeMap<String, Vec<String>>, check: &str, detail: impl Into<String>) {
    failures
        .entry(check.to_string())
        .or_default()
        .push(detail.into());
}

#[cfg(test)]
mod tests;
