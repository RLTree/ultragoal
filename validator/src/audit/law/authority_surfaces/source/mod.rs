use std::path::Path;

mod output;
mod raw;

pub(super) fn source_text_failures(root: &Path) -> Vec<(String, String)> {
    actual_source_files(root)
        .into_iter()
        .flat_map(|rel| {
            let text = std::fs::read_to_string(root.join(&rel)).unwrap_or_default();
            let mut out = raw::failures_for_text(&rel, &text)
                .into_iter()
                .map(|failure| ("typed-records-over-prose".to_string(), failure))
                .collect::<Vec<_>>();
            out.extend(
                output::failures_for_text(&rel, &text)
                    .into_iter()
                    .map(|failure| {
                        (
                            "total-authority-types-impossible-state-elimination".to_string(),
                            failure,
                        )
                    }),
            );
            out
        })
        .collect()
}

fn actual_source_files(root: &Path) -> Vec<String> {
    crate::package::inventory::closure::actual_files(root)
        .unwrap_or_default()
        .into_iter()
        .filter(|rel| rel.starts_with("validator/src/") && rel.ends_with(".rs"))
        .filter(|rel| !rel.contains("/self_tests/"))
        .collect()
}

#[cfg(test)]
pub(crate) fn raw_authority_failures_for_test(rel: &str, text: &str) -> Vec<String> {
    raw::failures_for_text(rel, text)
}

#[cfg(test)]
pub(crate) fn output_authority_failures_for_test(rel: &str, text: &str) -> Vec<String> {
    output::failures_for_text(rel, text)
}
