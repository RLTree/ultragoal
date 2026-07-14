mod output;
mod raw;
mod typed_boundary;

use std::path::Path;

pub(super) fn source_text_failures(root: &Path) -> Vec<(String, String)> {
    let audit = crate::audit::source_governance::audit(root);
    let inventory = audit.inventory;
    let mut failures = audit
        .failures
        .into_iter()
        .map(|failure| ("governed-source-inventory".to_string(), failure))
        .collect::<Vec<_>>();
    let production =
        crate::audit::source_governance::production_source::production_sources(&inventory);
    failures.extend(
        production
            .failures
            .into_iter()
            .map(|failure| ("production-source-classification".to_string(), failure)),
    );
    failures.extend(
        typed_boundary::failures(&production.sources)
            .into_iter()
            .map(|failure| ("typed-records-over-prose".to_string(), failure)),
    );
    for source in &production.sources {
        let Ok(text) = std::str::from_utf8(&source.bytes) else {
            failures.push((
                "total-authority-types-impossible-state-elimination".to_string(),
                format!("authority_source_non_utf8:{}", source.relative),
            ));
            continue;
        };
        failures.extend(
            raw::failures_for_text(&source.relative, text)
                .into_iter()
                .map(|failure| ("typed-records-over-prose".to_string(), failure)),
        );
        failures.extend(
            output::failures_for_text(&source.relative, text)
                .into_iter()
                .map(|failure| {
                    (
                        "total-authority-types-impossible-state-elimination".to_string(),
                        failure,
                    )
                }),
        );
    }
    failures.sort();
    failures.dedup();
    failures
}

#[cfg(test)]
pub(crate) fn raw_authority_failures_for_test(rel: &str, text: &str) -> Vec<String> {
    raw::failures_for_text(rel, text)
}

#[cfg(test)]
pub(crate) fn output_authority_failures_for_test(rel: &str, text: &str) -> Vec<String> {
    output::failures_for_text(rel, text)
}

#[cfg(test)]
pub(crate) use typed_boundary::BoundaryRow;
#[cfg(test)]
pub(crate) use typed_boundary::failures_for_sources_and_rows;
