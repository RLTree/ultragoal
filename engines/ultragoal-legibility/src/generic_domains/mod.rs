mod imports;
mod records;
#[cfg(test)]
mod records_tests;
mod source;
#[cfg(test)]
mod tests;
use crate::syntax::Report;
use std::collections::BTreeMap;

pub fn enforce(
    reports: &mut BTreeMap<String, Report>,
    files: &BTreeMap<String, Vec<u8>>,
) -> Vec<String> {
    let index = match source::Index::collect(files, reports.keys().cloned()) {
        Ok(index) => index,
        Err(error) => return vec![format!("generic_domain_source:{error}")],
    };
    let records = records::Records::new(&index, reports);
    let mut rejected = vec![];
    for (path, report) in reports.iter() {
        let Some(scope) = source::scope(path) else {
            continue;
        };
        for function in &report.functions {
            if function.generic.response_parameters.is_empty() {
                continue;
            }
            let verified = function
                .generic
                .response_parameters
                .iter()
                .all(|parameter| {
                    function.generic.constraints.iter().any(|(subject, bound)| {
                        (subject == parameter || subject == &function.generic.response)
                            && index
                                .resolve(&scope, bound)
                                .is_some_and(|marker| sealed(marker, &index, &records, &scope))
                    })
                });
            if !verified {
                rejected.push((path.clone(), function.name.clone()));
            }
        }
    }
    for (path, symbol) in rejected {
        let report = reports.get_mut(&path).unwrap();
        for function in report.functions.iter_mut().filter(|f| f.name == symbol) {
            function.returns_closed_result = false;
        }
        report.limitations.push(format!("generic_payload_bound_unverified:{symbol}:requires_owned_private_seal_with_finite_closed_implementations"));
    }
    vec![]
}

fn sealed(
    marker: &source::Trait,
    index: &source::Index,
    records: &records::Records<'_>,
    owner: &str,
) -> bool {
    if index
        .opaque_impl_packages
        .iter()
        .any(|p| Some(p.as_str()) == owner.split("::").next())
    {
        return false;
    }
    marker.supers.iter().any(|parent| {
        let Some(seal) = index.resolve(&marker.scope, parent) else {
            return false;
        };
        if !seal.private {
            return false;
        }
        let implementations: Vec<_> = index
            .implementations
            .iter()
            .filter(|i| {
                index
                    .resolve(&i.scope, &i.target)
                    .is_some_and(|t| t.key == seal.key)
            })
            .collect();
        !implementations.is_empty()
            && implementations
                .iter()
                .all(|i| !i.generic && records.closed(&i.ty, &i.scope))
    })
}
